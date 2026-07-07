//! Balance & robustness sweep: drive every authored encounter (all factory
//! heart-guards and every duelist party) through the real engine and assert
//! each reaches a definite outcome within a hard time cap. A battle that never
//! terminates would hang the game — this catches that across all content.

use crate::combat::engine::Battle;
use crate::combat::unit::UnitSpec;
use crate::combat::{BattleContext, RiderMods, Side, Stance};
use crate::data::settlement::DuelUnitDef;
use crate::data::GameData;

/// A strong, legal reference party a mid-late player might field: a walking
/// fortress plus a fast skirmisher, both well armed.
fn reference_player(data: &GameData) -> Vec<UnitSpec> {
    let bear = UnitSpec {
        species_id: "ferrobruin".to_owned(),
        name: "Ref Bear".to_owned(),
        side: Side::Player,
        creature_id: None,
        bond: 5.0,
        stance: Stance::Aggressive,
        grafts: vec![
            ("back".to_owned(), 0, "bolt_cannon".to_owned(), None),
            ("arm_l".to_owned(), 0, "ember_spitter".to_owned(), None),
            ("arm_r".to_owned(), 0, "basalt_carapace".to_owned(), None),
            ("haunch_l".to_owned(), 0, "shield_membrane".to_owned(), None),
        ],
    };
    let fox = UnitSpec {
        species_id: "volpi".to_owned(),
        name: "Ref Fox".to_owned(),
        side: Side::Player,
        creature_id: None,
        bond: 5.0,
        stance: Stance::Aggressive,
        grafts: vec![("foreleg_l".to_owned(), 0, "spark_coil".to_owned(), None)],
    };
    // Sanity: the reference party itself must be buildable.
    assert!(crate::combat::unit::BattleUnit::build(&bear, data, 0.0).is_ok());
    vec![bear, fox]
}

fn enemy_specs(name: &str, units: &[DuelUnitDef]) -> Vec<UnitSpec> {
    units
        .iter()
        .enumerate()
        .map(|(i, u)| UnitSpec {
            species_id: u.species.clone(),
            name: u
                .name
                .clone()
                .unwrap_or_else(|| format!("{} {}", name, i + 1)),
            side: Side::Enemy,
            creature_id: None,
            bond: 0.0,
            stance: Stance::Aggressive,
            grafts: u
                .grafts
                .iter()
                .map(|g| (g.limb.clone(), g.slot, g.graft.clone(), None))
                .collect(),
        })
        .collect()
}

/// Runs one battle headless (rider dismounted → pure standing-orders AI on
/// both sides) and returns whether it terminated.
fn terminates(data: &GameData, enemy: &[UnitSpec], context: BattleContext, seed: u64) -> bool {
    let player = reference_player(data);
    let mut battle = match Battle::new(data, context, &player, enemy, RiderMods::neutral(), seed) {
        Ok(b) => b,
        Err(_) => return false,
    };
    battle.rider.mounted_on = None;
    let dt = 0.05;
    // 600s of simulated combat is far beyond any legitimate encounter.
    for _ in 0..(600.0 / dt) as u32 {
        battle.update(data, dt);
        battle.drain_events();
        if battle.over() {
            return true;
        }
    }
    false
}

#[test]
fn every_factory_boss_terminates() {
    let data = GameData::load().unwrap();
    for (id, factory) in data.factories.iter() {
        let enemy = enemy_specs(&factory.name, &factory.heart_guard);
        // A few seeds each — RNG affects accuracy, strain, and salvage.
        for seed in [1u64, 7, 101] {
            assert!(
                terminates(&data, &enemy, BattleContext::FactoryDismantle, seed),
                "factory {} heart-guard did not terminate (seed {})",
                id,
                seed
            );
        }
    }
}

#[test]
fn every_duelist_party_terminates() {
    let data = GameData::load().unwrap();
    for (_, settlement) in data.settlements.iter() {
        for duelist in &settlement.duelists {
            let enemy = enemy_specs(&duelist.name, &duelist.party);
            for seed in [3u64, 55] {
                assert!(
                    terminates(&data, &enemy, BattleContext::Duel, seed),
                    "duelist {} did not terminate (seed {})",
                    duelist.id,
                    seed
                );
            }
        }
    }
}

/// The difficulty ramp should read: the reference party (mid-game gear)
/// reliably beats early unarmed wilds. Not a hard balance guarantee, just a
/// floor that early content is winnable.
#[test]
fn reference_party_clears_early_wilds() {
    let data = GameData::load().unwrap();
    let wilds = [
        vec![("bumblit", ""), ("bumblit", "")],
        vec![("thistlin", "")],
        vec![("quillow", ""), ("volpi", "")],
    ];
    for (i, pack) in wilds.iter().enumerate() {
        let enemy: Vec<UnitSpec> = pack
            .iter()
            .map(|(sp, _)| UnitSpec {
                species_id: (*sp).to_owned(),
                name: (*sp).to_owned(),
                side: Side::Enemy,
                creature_id: None,
                bond: 0.0,
                stance: Stance::Aggressive,
                grafts: vec![],
            })
            .collect();
        let player = reference_player(&data);
        let mut wins = 0;
        for seed in [2u64, 9, 40, 77] {
            let mut battle = Battle::new(
                &data,
                BattleContext::WildSubdue,
                &player,
                &enemy,
                RiderMods::neutral(),
                seed,
            )
            .unwrap();
            battle.rider.mounted_on = None;
            let dt = 0.05;
            for _ in 0..(600.0 / dt) as u32 {
                battle.update(&data, dt);
                battle.drain_events();
                if battle.over() {
                    break;
                }
            }
            if matches!(
                battle.outcome,
                Some(crate::combat::BattleOutcome::Victory(_))
            ) {
                wins += 1;
            }
        }
        assert!(
            wins >= 3,
            "reference party should clear early wild pack {} (won {}/4)",
            i,
            wins
        );
    }
}
