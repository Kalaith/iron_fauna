//! Presentation of battle events: which sound they make, whether they pause a
//! Wait-paced fight, and their one-line combat-log phrasing. Pure view logic,
//! kept out of the battle screen proper.

use crate::audio::Sfx;
use crate::combat::engine::Battle;
use crate::combat::events::BattleEvent;
use crate::combat::UnitId;
use crate::data::GameData;

/// Maps a battle event to its sound effect, if any.
pub(super) fn sfx_for_event(event: &BattleEvent) -> Option<Sfx> {
    Some(match event {
        BattleEvent::Hit { to_core, .. } => {
            if *to_core {
                Sfx::Crack
            } else {
                Sfx::Hit
            }
        }
        BattleEvent::LimbSevered { .. } => Sfx::Sever,
        BattleEvent::CoreCracked { .. } => Sfx::Crack,
        BattleEvent::HopStarted { .. } => Sfx::Hop,
        BattleEvent::GraftRejected { .. } | BattleEvent::BerserkStarted { .. } => Sfx::Reject,
        _ => return None,
    })
}

pub(super) fn describe_event(
    battle: &Battle,
    data: &GameData,
    event: &BattleEvent,
) -> Option<String> {
    let name = |id: &UnitId| battle.units[*id].spec_name.clone();
    let line = match event {
        BattleEvent::Hit {
            attacker,
            target,
            amount,
            to_core,
        } => {
            if *to_core {
                format!(
                    "{} strikes {}'s core for {:.0}!",
                    name(attacker),
                    name(target),
                    amount
                )
            } else {
                return None; // routine limb chip — too noisy for the log
            }
        }
        BattleEvent::Miss { attacker, target } => {
            format!("{} misses {}", name(attacker), name(target))
        }
        BattleEvent::LimbSevered { unit, limb_name } => {
            format!("{} loses its {}!", name(unit), limb_name)
        }
        BattleEvent::LimbRegrown { unit, limb_name } => {
            format!("{}'s {} regrows", name(unit), limb_name)
        }
        BattleEvent::GraftDestroyed { unit, graft_name } => {
            format!("{}'s {} is destroyed", name(unit), graft_name)
        }
        BattleEvent::GraftRejected { unit, graft_name } => {
            format!("{} REJECTS its {}!", name(unit), graft_name)
        }
        BattleEvent::SalvageDropped { def_id } => {
            let n = data
                .graftware
                .get(def_id)
                .map(|d| d.name.clone())
                .unwrap_or_else(|| def_id.clone());
            format!("{} clatters to the ground", n)
        }
        BattleEvent::CoreExposed { unit } => format!("{}'s core is EXPOSED", name(unit)),
        BattleEvent::CoreCracked { unit } => {
            format!("{}'s core cracks — it's over for it", name(unit))
        }
        BattleEvent::BerserkStarted { unit } => format!("{} goes BERSERK!", name(unit)),
        BattleEvent::BerserkEnded { unit } => format!("{} calms", name(unit)),
        BattleEvent::HopStarted { to, .. } => format!("rider leaps toward {}", name(to)),
        BattleEvent::HopLanded { to } => format!("rider mounts {}", name(to)),
        BattleEvent::RiderExposed => "the rider is thrown into the open!".to_owned(),
        BattleEvent::Healed { target, amount, .. } => {
            format!("{} is soothed for {:.0}", name(target), amount)
        }
        BattleEvent::Shielded { unit, amount } => {
            format!("{}'s core is shielded (+{:.0})", name(unit), amount)
        }
        BattleEvent::ItemUsed { unit, label } => format!("{} — {}", name(unit), label),
        BattleEvent::StanceChanged { unit } => format!(
            "{} switches to {}",
            name(unit),
            battle.units[*unit].stance.display_name()
        ),
        BattleEvent::RiddenActionReady { .. } => return None,
        BattleEvent::BattleEnded => return None,
    };
    Some(line)
}
