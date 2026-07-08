//! Player commands and attack/effect resolution.

use crate::combat::engine::Battle;
use crate::combat::events::BattleEvent;
use crate::combat::unit::Dot;
use crate::combat::{CalledTarget, PlayerCommand, Side, UnitId, WeaponRef};
use crate::data::graftware::{BoostEffect, GraftEffect};
use crate::data::item::ConsumableEffect;
use crate::data::GameData;

impl Battle {
    /// Issues a player command. Returns false if the command was invalid
    /// (wrong state, on cooldown, out of vigor/range).
    pub fn command(&mut self, data: &GameData, cmd: PlayerCommand) -> bool {
        if self.over() {
            return false;
        }
        match cmd {
            PlayerCommand::SetStance { unit, stance } => {
                let Some(u) = self.units.get_mut(unit) else {
                    return false;
                };
                if u.side != Side::Player || !u.alive() {
                    return false;
                }
                u.stance = stance;
                self.events.push(BattleEvent::StanceChanged { unit });
                true
            }
            PlayerCommand::BeginHop { to } => self.begin_hop(data, to),
            other => {
                let Some(id) = self.ridden_unit() else {
                    return false;
                };
                if !self.is_commandable(id) {
                    return false;
                }
                self.ridden_command(data, id, other)
            }
        }
    }

    fn ridden_command(&mut self, data: &GameData, id: UnitId, cmd: PlayerCommand) -> bool {
        match cmd {
            PlayerCommand::Fire {
                mount,
                target,
                called,
            } => {
                let ok = self.try_attack(data, id, target, WeaponRef::Mount(mount), called, true);
                if ok {
                    self.ridden_ready_announced = false;
                }
                ok
            }
            PlayerCommand::NaturalAttack { target, called } => {
                let ok = self.try_attack(data, id, target, WeaponRef::Natural, called, true);
                if ok {
                    self.ridden_ready_announced = false;
                }
                ok
            }
            PlayerCommand::TriggerUtility { mount, ally } => {
                self.trigger_utility(data, id, mount, ally)
            }
            PlayerCommand::Regrow { limb } => self.begin_regrow(id, limb),
            PlayerCommand::Reinforce => self.reinforce(data, id),
            PlayerCommand::SetStance { .. } | PlayerCommand::BeginHop { .. } => false,
        }
    }

    /// Rider-hop (`combat.md` §5): a real, costed action — the rider is
    /// exposed mid-transit and both creatures fall back to standing orders.
    pub(crate) fn begin_hop(&mut self, data: &GameData, to: UnitId) -> bool {
        if self.rider.hop.is_some() {
            return false;
        }
        let valid = self
            .units
            .get(to)
            .is_some_and(|u| u.side == Side::Player && u.alive());
        if !valid || self.rider.mounted_on == Some(to) {
            return false;
        }
        let from = self.rider.mounted_on;
        let time = data.balance.battle.hop_transit_time * self.rider_mods.hop_time_mult;
        self.rider.mounted_on = None;
        self.rider.hop = Some((to, time));
        self.events.push(BattleEvent::HopStarted { from, to });
        true
    }

    pub(crate) fn begin_regrow(&mut self, id: UnitId, limb: usize) -> bool {
        let u = &mut self.units[id];
        let Some(l) = u.limbs.get(limb) else {
            return false;
        };
        if !l.severed {
            return false;
        }
        u.regrow_target = Some(limb);
        true
    }

    pub(crate) fn reinforce(&mut self, data: &GameData, id: UnitId) -> bool {
        let bal = &data.balance;
        let ridden = self.ridden_unit() == Some(id);
        let mult = if ridden {
            self.rider_mods.reinforce_mult
        } else {
            1.0
        };
        let u = &mut self.units[id];
        if u.reinforce_cooldown > 0.0 || u.vigor < bal.vigor.reinforce_cost {
            return false;
        }
        u.vigor -= bal.vigor.reinforce_cost;
        u.reinforce_cooldown = bal.vigor.reinforce_cooldown;
        let cap = u.core_max * bal.vigor.reinforce_shield_cap_frac;
        let amount = bal.vigor.reinforce_shield * mult;
        u.shield = (u.shield + amount).min(cap);
        self.events.push(BattleEvent::Shielded { unit: id, amount });
        true
    }

    pub(crate) fn trigger_utility(
        &mut self,
        data: &GameData,
        id: UnitId,
        mount: usize,
        ally: Option<UnitId>,
    ) -> bool {
        let ridden = self.ridden_unit() == Some(id);
        let (effect, synergy) = {
            let u = &self.units[id];
            let Some(m) = u.mounts.get(mount) else {
                return false;
            };
            if !m.usable() || !u.limbs[m.limb_index].intact() || m.cooldown > 0.0 {
                return false;
            }
            let Some(def) = data.graftware.get(&m.def_id) else {
                return false;
            };
            let Some(effect) = def.effect else {
                return false;
            };
            (effect, u.synergy(data, def))
        };
        let amplify = if ridden {
            self.ridden_boosts(data, id)
                .filter_map(|b| match b {
                    BoostEffect::Amplify { mult } => Some(mult),
                    _ => None,
                })
                .product::<f32>()
                .max(1.0)
        } else {
            1.0
        };

        match effect {
            GraftEffect::Heal {
                amount,
                cooldown,
                vigor_cost,
            } => {
                if self.units[id].vigor < vigor_cost {
                    return false;
                }
                let target = ally
                    .filter(|&a| {
                        self.units
                            .get(a)
                            .is_some_and(|u| u.side == Side::Player && u.alive())
                    })
                    .unwrap_or(id);
                self.units[id].vigor -= vigor_cost;
                self.units[id].mounts[mount].cooldown = cooldown;
                let healed = amount * synergy * amplify;
                self.heal_unit(target, healed);
                self.events.push(BattleEvent::Healed {
                    source: id,
                    target,
                    amount: healed,
                });
                true
            }
            GraftEffect::ShieldCore {
                amount,
                cooldown,
                vigor_cost,
            } => {
                let bal = &data.balance;
                let u = &mut self.units[id];
                if u.vigor < vigor_cost {
                    return false;
                }
                u.vigor -= vigor_cost;
                u.mounts[mount].cooldown = cooldown;
                let cap = u.core_max * bal.vigor.reinforce_shield_cap_frac;
                let gained = amount * synergy * amplify;
                u.shield = (u.shield + gained).min(cap);
                self.events.push(BattleEvent::Shielded {
                    unit: id,
                    amount: gained,
                });
                true
            }
            // Passive effects have no trigger.
            _ => false,
        }
    }

    pub(crate) fn heal_unit(&mut self, id: UnitId, amount: f32) {
        let u = &mut self.units[id];
        // Heal the most-wounded intact limb; overflow soothes the core.
        let target = u
            .limbs
            .iter_mut()
            .filter(|l| l.intact() && l.hp < l.max_hp)
            .min_by(|a, b| {
                (a.hp / a.max_hp)
                    .partial_cmp(&(b.hp / b.max_hp))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        if let Some(limb) = target {
            limb.hp = (limb.hp + amount).min(limb.max_hp);
        } else {
            u.core_hp = (u.core_hp + amount * 0.5).min(u.core_max);
        }
    }

    /// The damage multiplier and optional burn from the ammo loaded in a
    /// weapon (`(1.0, None)` for natural attacks or standard fire).
    fn ammo_effect(
        &self,
        data: &GameData,
        attacker: UnitId,
        weapon: WeaponRef,
    ) -> (f32, Option<(f32, f32)>) {
        let WeaponRef::Mount(m) = weapon else {
            return (1.0, None);
        };
        let Some(ammo) = self.units[attacker]
            .mounts
            .get(m)
            .and_then(|mt| mt.ammo.as_ref())
        else {
            return (1.0, None);
        };
        match data.items.get(&ammo.def_id).map(|d| d.effect) {
            Some(ConsumableEffect::Ammo {
                damage_mult,
                burn_dps,
                burn_secs,
                ..
            }) => (
                damage_mult,
                (burn_dps > 0.0).then_some((burn_dps, burn_secs)),
            ),
            _ => (1.0, None),
        }
    }

    /// Attempts an attack; validates range, cooldown, and vigor.
    pub(crate) fn try_attack(
        &mut self,
        data: &GameData,
        attacker: UnitId,
        target: UnitId,
        weapon: WeaponRef,
        called: Option<CalledTarget>,
        is_player_call: bool,
    ) -> bool {
        let bal = &data.balance;
        if !self.units.get(target).is_some_and(|u| u.alive()) {
            return false;
        }
        if self.units[attacker].side == self.units[target].side {
            return false;
        }

        // Gather weapon parameters. With fixed positions every foe is in reach;
        // weapons differ by damage, cooldown, and cost, not range.
        let (damage, vigor_cost, cooldown, synergy, draw, boost_active) = match weapon {
            WeaponRef::Natural => {
                let u = &self.units[attacker];
                if u.natural_cooldown > 0.0 {
                    return false;
                }
                (
                    u.natural_damage,
                    2.0,
                    bal.battle.natural_attack_cooldown,
                    1.0,
                    0.0,
                    false,
                )
            }
            WeaponRef::Mount(m) => {
                let u = &self.units[attacker];
                let Some(mount) = u.mounts.get(m) else {
                    return false;
                };
                if !mount.usable() || !u.limbs[mount.limb_index].intact() || mount.cooldown > 0.0 {
                    return false;
                }
                let Some(def) = data.graftware.get(&mount.def_id) else {
                    return false;
                };
                if !def.is_weapon() {
                    return false;
                }
                let ridden = self.ridden_unit() == Some(attacker);
                (
                    def.damage,
                    def.vigor_cost,
                    def.cooldown,
                    u.synergy(data, def),
                    def.power_draw as f32,
                    ridden,
                )
            }
        };

        if self.units[attacker].vigor < vigor_cost {
            return false;
        }

        // Loaded ammunition modifies this shot (`combat.md` §3.3).
        let (ammo_mult, ammo_burn) = self.ammo_effect(data, attacker, weapon);

        // Pay costs up front.
        {
            let u = &mut self.units[attacker];
            u.vigor -= vigor_cost;
            match weapon {
                WeaponRef::Natural => u.natural_cooldown = cooldown,
                WeaponRef::Mount(m) => u.mounts[m].cooldown = cooldown,
            }
            // Firing hard weapons strains the host (`game_design.md` §4.3).
            u.strain += draw * bal.strain.fire_gain_per_draw;
        }
        // A fired round is spent whether it lands or not.
        if let WeaponRef::Mount(m) = weapon {
            if let Some(ammo) = &mut self.units[attacker].mounts[m].ammo {
                ammo.rounds = ammo.rounds.saturating_sub(1);
                if ammo.rounds == 0 {
                    self.units[attacker].mounts[m].ammo = None;
                }
            }
        }

        // Accuracy roll.
        let mut acc =
            bal.battle.base_accuracy + self.units[attacker].effective_accuracy_bonus(data);
        if called.is_some() {
            acc *= bal.battle.called_shot_accuracy_mult;
            if is_player_call {
                acc *= self.rider_mods.called_shot_mult;
            }
        }
        let hit_chance = (acc * (1.0 - self.units[target].dodge)).clamp(0.4, 0.97);
        if !self.rng.chance(hit_chance) {
            self.events.push(BattleEvent::Miss { attacker, target });
            return true; // the shot happened; it just missed
        }

        let dealt = damage * synergy * ammo_mult * bal.battle.weapon_damage_mult;
        self.apply_damage(data, attacker, target, dealt, called);
        // Incendiary ammo leaves a burn on the struck body.
        if let Some((dps, secs)) = ammo_burn {
            if self.units[target].alive() {
                let intact = self.units[target].intact_limbs();
                if intact.is_empty() {
                    self.units[target].core_dots.push(Dot {
                        dps,
                        remaining: secs,
                    });
                } else {
                    let li = intact[self.rng.below(intact.len())];
                    self.units[target].limb_dots.push((
                        li,
                        Dot {
                            dps,
                            remaining: secs,
                        },
                    ));
                }
            }
        }

        // Ridden boosts that ride along on weapon hits (`combat.md` §3.2).
        if boost_active {
            let boosts: Vec<BoostEffect> = self.ridden_boosts(data, attacker).collect();
            for b in boosts {
                match b {
                    BoostEffect::Corrode { dps, duration } => {
                        self.units[target].core_dots.push(crate::combat::unit::Dot {
                            dps,
                            remaining: duration,
                        });
                    }
                    BoostEffect::ChainArc {
                        extra_targets,
                        falloff,
                    } => {
                        let mut chained = 0;
                        let others = self.alive_on(self.units[target].side);
                        for other in others {
                            if other != target && chained < extra_targets {
                                chained += 1;
                                self.apply_damage(data, attacker, other, dealt * falloff, None);
                            }
                        }
                    }
                    BoostEffect::Barrage { extra_shots } => {
                        for _ in 0..extra_shots {
                            if self.units[target].alive() {
                                self.apply_damage(data, attacker, target, dealt * 0.5, None);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        true
    }

    /// Routes damage per the anatomy: called mount → graft; otherwise a limb;
    /// once all limbs are gone, the exposed core.
    pub(crate) fn apply_damage(
        &mut self,
        data: &GameData,
        attacker: UnitId,
        target: UnitId,
        amount: f32,
        called: Option<CalledTarget>,
    ) {
        if !self.units[target].alive() {
            return;
        }
        let spill_frac = data.balance.battle.graft_spill_frac;

        match called {
            Some(CalledTarget::Mount(mi))
                if self.units[target].mounts.get(mi).is_some_and(|m| {
                    m.usable() && self.units[target].limbs[m.limb_index].intact()
                }) =>
            {
                // Chip the graft itself; spill wounds the host limb.
                let (destroyed, spill, limb_index) = {
                    let m = &mut self.units[target].mounts[mi];
                    m.graft_hp -= amount;
                    let destroyed = m.graft_hp <= 0.0;
                    let spill = if destroyed {
                        (-m.graft_hp).max(0.0) * spill_frac
                    } else {
                        amount * spill_frac * 0.5
                    };
                    (destroyed, spill, m.limb_index)
                };
                self.events.push(BattleEvent::Hit {
                    attacker,
                    target,
                    amount,
                    to_core: false,
                });
                if destroyed {
                    self.destroy_mount(data, target, mi);
                }
                if spill > 0.0 {
                    self.damage_limb_raw(data, target, limb_index, spill);
                }
            }
            Some(CalledTarget::Limb(li))
                if self.units[target].limbs.get(li).is_some_and(|l| l.intact()) =>
            {
                self.events.push(BattleEvent::Hit {
                    attacker,
                    target,
                    amount,
                    to_core: false,
                });
                self.damage_limb_armored(data, target, li, amount);
            }
            _ => {
                let intact = self.units[target].intact_limbs();
                if intact.is_empty() {
                    // Core exposed: shield, then the core itself.
                    let dealt = amount * data.balance.battle.exposed_core_damage_mult;
                    self.events.push(BattleEvent::Hit {
                        attacker,
                        target,
                        amount: dealt,
                        to_core: true,
                    });
                    self.damage_core_raw(target, dealt);
                } else {
                    let li = intact[self.rng.below(intact.len())];
                    self.events.push(BattleEvent::Hit {
                        attacker,
                        target,
                        amount,
                        to_core: false,
                    });
                    self.damage_limb_armored(data, target, li, amount);
                }
            }
        }
    }

    fn damage_limb_armored(&mut self, data: &GameData, target: UnitId, li: usize, amount: f32) {
        let armor = self.units[target].limb_armor(data, li);
        // Armor can't fully negate — a fifth always bleeds through.
        let dealt = (amount - armor).max(amount * 0.2);
        self.damage_limb_raw(data, target, li, dealt);
    }

    pub(crate) fn damage_limb_raw(
        &mut self,
        data: &GameData,
        target: UnitId,
        li: usize,
        amount: f32,
    ) {
        let severed = {
            let limb = &mut self.units[target].limbs[li];
            if !limb.intact() {
                return;
            }
            limb.hp -= amount;
            limb.hp <= 0.0
        };
        if severed {
            self.sever_limb(data, target, li);
        }
    }

    /// Blowing off a limb detaches its graftware as salvage
    /// (`game_design.md` §6 — salvage economy).
    fn sever_limb(&mut self, data: &GameData, target: UnitId, li: usize) {
        {
            let limb = &mut self.units[target].limbs[li];
            limb.severed = true;
            limb.hp = 0.0;
            limb.regrow_hp = 0.0;
        }
        let limb_name = {
            let u = &self.units[target];
            u.limb_def(data, &u.limbs[li]).name.clone()
        };
        self.events.push(BattleEvent::LimbSevered {
            unit: target,
            limb_name,
        });
        self.units[target].limb_dots.retain(|(l, _)| *l != li);

        let enemy_side = self.units[target].side == Side::Enemy;
        let salvage_chance =
            data.balance.battle.salvage_drop_chance + self.rider_mods.salvage_bonus;
        let mount_ids: Vec<usize> = self.units[target]
            .mounts
            .iter()
            .enumerate()
            .filter(|(_, m)| m.limb_index == li && m.usable())
            .map(|(i, _)| i)
            .collect();
        for mi in mount_ids {
            let def_id = self.units[target].mounts[mi].def_id.clone();
            self.units[target].mounts[mi].detached = true;
            if enemy_side && self.rng.chance(salvage_chance) {
                self.salvage.push(def_id.clone());
                self.events.push(BattleEvent::SalvageDropped { def_id });
            }
        }

        if self.units[target].core_exposed() {
            self.events.push(BattleEvent::CoreExposed { unit: target });
        }
    }

    fn destroy_mount(&mut self, data: &GameData, target: UnitId, mi: usize) {
        let name = self.graft_name(data, target, mi);
        self.units[target].mounts[mi].destroyed = true;
        self.events.push(BattleEvent::GraftDestroyed {
            unit: target,
            graft_name: name,
        });
    }

    pub(crate) fn damage_core_raw(&mut self, target: UnitId, amount: f32) {
        let cracked = {
            let u = &mut self.units[target];
            let after_shield = (amount - u.shield).max(0.0);
            u.shield = (u.shield - amount).max(0.0);
            u.core_hp -= after_shield;
            u.core_hp <= 0.0
        };
        if cracked {
            self.crack_core(target);
        }
    }

    /// Cracking the core downs the creature — a capture, a freed core, or a
    /// yield; never a kill (`game_design.md` §4.1).
    fn crack_core(&mut self, target: UnitId) {
        let u = &mut self.units[target];
        u.downed = true;
        u.core_hp = 0.0;
        self.events.push(BattleEvent::CoreCracked { unit: target });

        if self.rider.mounted_on == Some(target) {
            self.rider.mounted_on = None;
            self.events.push(BattleEvent::RiderExposed);
        }
    }
}
