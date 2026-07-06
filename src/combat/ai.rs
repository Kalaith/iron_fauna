//! Standing-orders AI (`combat.md` §4): two legible stances plus cooldowns.
//! Aggressive fires whatever's off cooldown at the nearest valid target;
//! Defensive conserves, guards the core, and rebuilds. Enemies run the same
//! rules — no hidden "smart AI".

use crate::combat::engine::Battle;
use crate::combat::unit::BattleUnit;
use crate::combat::{Stance, UnitId, WeaponRef};
use crate::data::graftware::{GraftEffect, RangeBand};
use crate::data::GameData;

pub fn think(battle: &mut Battle, data: &GameData, id: UnitId) {
    let Some(target) = nearest_opponent(battle, id) else {
        battle.units[id].move_intent = 0.0;
        return;
    };

    let stance = if battle.units[id].berserk() {
        Stance::Aggressive
    } else {
        battle.units[id].stance
    };

    set_movement(battle, data, id, target, stance);
    act(battle, data, id, target, stance);
}

fn nearest_opponent(battle: &Battle, id: UnitId) -> Option<UnitId> {
    let me = &battle.units[id];
    battle
        .alive_on(me.side.opponent())
        .into_iter()
        .min_by(|&a, &b| {
            let da = battle.units[a].distance_to(me);
            let db = battle.units[b].distance_to(me);
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        })
}

/// The range this unit wants to fight at: its best usable weapon's band,
/// or melee reach when unarmed.
fn preferred_range(unit: &BattleUnit, data: &GameData) -> f32 {
    let bal = &data.balance.battle;
    unit.weapon_mounts(data)
        .into_iter()
        .filter_map(|m| data.graftware.get(&unit.mounts[m].def_id))
        .map(|def| match def.range {
            RangeBand::Melee => bal.melee_range,
            RangeBand::Short => bal.short_range,
            RangeBand::Long => bal.long_range,
        })
        .fold(None::<f32>, |acc, r| Some(acc.map_or(r, |a| a.max(r))))
        .unwrap_or(bal.melee_range)
}

fn set_movement(battle: &mut Battle, data: &GameData, id: UnitId, target: UnitId, stance: Stance) {
    let dist = battle.units[id].distance_to(&battle.units[target]);
    let toward = (battle.units[target].pos - battle.units[id].pos).signum();
    let reach = preferred_range(&battle.units[id], data);

    let intent = match stance {
        Stance::Aggressive => {
            if dist > reach * 0.9 {
                toward
            } else if dist < reach * 0.4 && reach > data.balance.battle.melee_range {
                -toward * 0.5 // back off to firing distance
            } else {
                0.0
            }
        }
        Stance::Defensive => {
            if dist < reach * 0.8 {
                -toward // keep the enemy at arm's length
            } else if dist > reach {
                toward * 0.4
            } else {
                0.0
            }
        }
    };
    battle.units[id].move_intent = intent;
}

fn act(battle: &mut Battle, data: &GameData, id: UnitId, target: UnitId, stance: Stance) {
    // Defensive priorities: shield the core, regrow, and only then shoot.
    if stance == Stance::Defensive {
        try_defensive_upkeep(battle, data, id);
    }

    // Support utilities fire in both stances when useful.
    try_utilities(battle, data, id);

    // Regrow: aggressive units rebuild once badly stripped; defensive always.
    let severed: Vec<usize> = battle.units[id]
        .limbs
        .iter()
        .enumerate()
        .filter(|(_, l)| l.severed)
        .map(|(i, _)| i)
        .collect();
    if battle.units[id].regrow_target.is_none() && !severed.is_empty() {
        let limb_count = battle.units[id].limbs.len();
        let should = match stance {
            Stance::Defensive => true,
            Stance::Aggressive => severed.len() * 2 >= limb_count,
        };
        if should {
            battle.begin_regrow(id, severed[0]);
        }
    }

    // Fire whatever's ready. Defensive units hold fire below half vigor.
    let vigor_floor = match stance {
        Stance::Aggressive => 0.0,
        Stance::Defensive => battle.units[id].vigor_max * 0.5,
    };
    if battle.units[id].vigor > vigor_floor {
        let mounts = battle.units[id].weapon_mounts(data);
        for m in mounts {
            if battle.units[id].mounts[m].cooldown <= 0.0 {
                battle.try_attack(data, id, target, WeaponRef::Mount(m), None, false);
            }
        }
    }
    // Natural melee when close, always.
    if battle.units[id].natural_cooldown <= 0.0 {
        battle.try_attack(data, id, target, WeaponRef::Natural, None, false);
    }
}

fn try_defensive_upkeep(battle: &mut Battle, data: &GameData, id: UnitId) {
    let u = &battle.units[id];
    let shield_low = u.shield < data.balance.vigor.reinforce_shield * 0.5;
    let threatened = u.intact_limbs().len() <= 1 || u.core_hp < u.core_max;
    if shield_low && threatened {
        battle.reinforce(data, id);
    }
}

fn try_utilities(battle: &mut Battle, data: &GameData, id: UnitId) {
    let mount_count = battle.units[id].mounts.len();
    for m in 0..mount_count {
        let (ready, effect) = {
            let u = &battle.units[id];
            let mount = &u.mounts[m];
            let ready =
                mount.usable() && u.limbs[mount.limb_index].intact() && mount.cooldown <= 0.0;
            let effect = data.graftware.get(&mount.def_id).and_then(|d| d.effect);
            (ready, effect)
        };
        if !ready {
            continue;
        }
        match effect {
            Some(GraftEffect::Heal { .. }) => {
                if let Some(ally) = most_wounded_ally(battle, id) {
                    battle.trigger_utility(data, id, m, Some(ally));
                }
            }
            Some(GraftEffect::ShieldCore { .. }) => {
                let u = &battle.units[id];
                let endangered = u.intact_limbs().len() <= 2 && u.shield <= 0.0;
                if endangered {
                    battle.trigger_utility(data, id, m, None);
                }
            }
            _ => {}
        }
    }
}

/// The friendly unit (possibly self) with the lowest limb-health fraction,
/// if anyone is actually hurt.
fn most_wounded_ally(battle: &Battle, id: UnitId) -> Option<UnitId> {
    let side = battle.units[id].side;
    battle
        .alive_on(side)
        .into_iter()
        .filter_map(|a| {
            let u = &battle.units[a];
            let total_max: f32 = u.limbs.iter().map(|l| l.max_hp).sum();
            let total_hp: f32 = u.limbs.iter().map(|l| l.hp.max(0.0)).sum();
            let frac = if total_max > 0.0 {
                total_hp / total_max
            } else {
                1.0
            };
            (frac < 0.7).then_some((a, frac))
        })
        .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(a, _)| a)
}
