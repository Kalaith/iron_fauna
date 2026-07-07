//! Whole-battle simulation tests: run the engine headless and assert the
//! design-level invariants hold.

use crate::combat::engine::Battle;
use crate::combat::events::BattleEvent;
use crate::combat::unit::UnitSpec;
use crate::combat::{
    BattleContext, BattleOutcome, CalledTarget, PlayerCommand, RiderMods, Side, Stance, WeaponRef,
};
use crate::data::GameData;

fn spec(species: &str, side: Side, grafts: Vec<(&str, usize, &str)>) -> UnitSpec {
    UnitSpec {
        species_id: species.to_owned(),
        name: species.to_owned(),
        side,
        creature_id: None,
        bond: 0.0,
        stance: Stance::Aggressive,
        grafts: grafts
            .into_iter()
            .map(|(l, s, d)| (l.to_owned(), s, d.to_owned(), None))
            .collect(),
    }
}

fn run_to_outcome(battle: &mut Battle, data: &GameData, max_seconds: f32) -> Vec<BattleEvent> {
    let mut all_events = Vec::new();
    let dt = 0.05;
    let mut t = 0.0;
    while !battle.over() && t < max_seconds {
        battle.update(data, dt);
        all_events.extend(battle.drain_events());
        t += dt;
    }
    all_events
}

#[test]
fn called_shot_silences_the_targeted_graft() {
    let data = GameData::load().unwrap();
    // Player Bear with a long cannon vs an enemy Pangol carrying an ember
    // spitter on its left arm. A called shot at that mount should destroy it
    // specifically, not sever a random limb.
    let mut battle = Battle::new(
        &data,
        BattleContext::FactoryDismantle,
        &[spec(
            "ferrobruin",
            Side::Player,
            vec![("back", 0, "bolt_cannon")],
        )],
        &[spec(
            "pangol",
            Side::Enemy,
            vec![("arm_l", 0, "ember_spitter")],
        )],
        RiderMods::neutral(),
        4,
    )
    .unwrap();
    // Close the range so the shot connects, and find the enemy's graft mount.
    battle.units[1].pos = battle.units[0].pos + 100.0;
    let mount = battle.units[1]
        .mounts
        .iter()
        .position(|m| m.def_id == "ember_spitter")
        .unwrap();
    let limb_index = battle.units[1].mounts[mount].limb_index;

    // Hammer the mount with called shots until it breaks (accuracy can miss).
    let mut destroyed = false;
    for _ in 0..60 {
        battle.units[0].vigor = battle.units[0].vigor_max;
        battle.units[0].mounts[0].cooldown = 0.0;
        battle.try_attack(
            &data,
            0,
            1,
            WeaponRef::Mount(0),
            Some(CalledTarget::Mount(mount)),
            true,
        );
        if battle.units[1].mounts[mount].destroyed {
            destroyed = true;
            break;
        }
        // The limb hosting it must not have been severed out from under the
        // called shot — the graft is what we aimed at.
        assert!(
            battle.units[1].limbs[limb_index].intact() || battle.units[1].mounts[mount].destroyed,
            "called shot severed the limb instead of silencing the graft"
        );
    }
    assert!(destroyed, "called shots never silenced the targeted graft");
}

#[test]
fn armed_party_subdues_wild_creature() {
    let data = GameData::load().unwrap();
    let mut battle = Battle::new(
        &data,
        BattleContext::WildSubdue,
        &[
            spec("volpi", Side::Player, vec![("foreleg_l", 0, "spark_coil")]),
            spec("pangol", Side::Player, vec![("arm_l", 0, "ember_spitter")]),
        ],
        &[spec("bumblit", Side::Enemy, vec![])],
        RiderMods::neutral(),
        7,
    )
    .unwrap();
    // Rider dismounted: every player unit fights on standing orders.
    battle.rider.mounted_on = None;

    let events = run_to_outcome(&mut battle, &data, 300.0);

    let Some(BattleOutcome::Victory(rewards)) = battle.outcome.clone() else {
        panic!("expected victory, got {:?}", battle.outcome);
    };
    assert_eq!(rewards.captured_species, vec!["bumblit".to_owned()]);
    assert!(rewards.scrip > 0);
    // The wild core was exposed before it was cracked — never skipped.
    let exposed_at = events
        .iter()
        .position(|e| matches!(e, BattleEvent::CoreExposed { unit: 2 }));
    let cracked_at = events
        .iter()
        .position(|e| matches!(e, BattleEvent::CoreCracked { unit: 2 }));
    assert!(exposed_at.is_some() && cracked_at.is_some());
    assert!(exposed_at.unwrap() < cracked_at.unwrap());
}

#[test]
fn duel_yield_captures_nothing() {
    let data = GameData::load().unwrap();
    let mut battle = Battle::new(
        &data,
        BattleContext::Duel,
        &[spec(
            "ferrobruin",
            Side::Player,
            vec![("back", 0, "bolt_cannon"), ("arm_l", 0, "ember_spitter")],
        )],
        &[spec("bumblit", Side::Enemy, vec![])],
        RiderMods::neutral(),
        99,
    )
    .unwrap();
    battle.rider.mounted_on = None;

    run_to_outcome(&mut battle, &data, 300.0);
    let Some(BattleOutcome::Victory(rewards)) = battle.outcome.clone() else {
        panic!("expected duel victory, got {:?}", battle.outcome);
    };
    assert!(rewards.captured_species.is_empty(), "duels never capture");
}

#[test]
fn losing_every_core_means_fleeing_not_game_over() {
    let data = GameData::load().unwrap();
    let mut battle = Battle::new(
        &data,
        BattleContext::FactoryDismantle,
        &[spec("bumblit", Side::Player, vec![])],
        &[
            spec(
                "ferrobruin",
                Side::Enemy,
                vec![("back", 0, "bolt_cannon"), ("arm_l", 0, "ember_spitter")],
            ),
            spec("pangol", Side::Enemy, vec![("arm_l", 0, "ember_spitter")]),
        ],
        RiderMods::neutral(),
        3,
    )
    .unwrap();
    battle.rider.mounted_on = None;

    run_to_outcome(&mut battle, &data, 300.0);
    assert_eq!(battle.outcome, Some(BattleOutcome::Fled));
}

#[test]
fn commands_respect_riding_rules() {
    let data = GameData::load().unwrap();
    let mut battle = Battle::new(
        &data,
        BattleContext::WildSubdue,
        &[
            spec("volpi", Side::Player, vec![("foreleg_l", 0, "spark_coil")]),
            spec("pangol", Side::Player, vec![]),
        ],
        &[spec("bumblit", Side::Enemy, vec![])],
        RiderMods::neutral(),
        11,
    )
    .unwrap();

    // Stance orders reach any friendly unit.
    assert!(battle.command(
        &data,
        PlayerCommand::SetStance {
            unit: 1,
            stance: Stance::Defensive
        }
    ));
    // Hopping to the mount you're already on is refused.
    assert!(!battle.command(&data, PlayerCommand::BeginHop { to: 0 }));
    // Hopping to an enemy is refused.
    assert!(!battle.command(&data, PlayerCommand::BeginHop { to: 2 }));
    // A real hop dismounts and transits; mid-hop there is no ridden unit.
    assert!(battle.command(&data, PlayerCommand::BeginHop { to: 1 }));
    assert!(battle.ridden_unit().is_none());
    assert!(!battle.rider.exposed(), "in transit, not exposed");
    // Firing while mid-hop is refused (no mount under the rider).
    assert!(!battle.command(
        &data,
        PlayerCommand::NaturalAttack {
            target: 2,
            called: None
        }
    ));
    // Land the hop.
    for _ in 0..40 {
        battle.update(&data, 0.05);
    }
    assert_eq!(battle.ridden_unit(), Some(1));
}

#[test]
fn strain_at_threshold_breaks_the_creature() {
    let data = GameData::load().unwrap();
    let mut battle = Battle::new(
        &data,
        BattleContext::WildSubdue,
        &[spec(
            "volpi",
            Side::Player,
            vec![("foreleg_l", 0, "spark_coil")],
        )],
        &[spec("bumblit", Side::Enemy, vec![])],
        RiderMods::neutral(),
        21,
    )
    .unwrap();

    // Force the ridden creature to the brink.
    battle.units[0].strain = battle.units[0].strain_threshold * 1.2;
    battle.units[0].strain_check_accum = 0.99;
    battle.update(&data, 0.05);
    let events = battle.drain_events();
    let broke = events.iter().any(|e| {
        matches!(
            e,
            BattleEvent::BerserkStarted { unit: 0 } | BattleEvent::GraftRejected { unit: 0, .. }
        )
    });
    assert!(broke, "expected berserk or rejection, got {:?}", events);
}

#[test]
fn severing_an_armed_enemy_limb_can_drop_salvage() {
    let data = GameData::load().unwrap();
    // Deterministic seed chosen so at least one salvage roll succeeds across
    // the enemy's armed limbs being stripped.
    let mut battle = Battle::new(
        &data,
        BattleContext::FactoryDismantle,
        &[
            spec(
                "ferrobruin",
                Side::Player,
                vec![("back", 0, "bolt_cannon"), ("arm_l", 0, "ember_spitter")],
            ),
            spec("volpi", Side::Player, vec![("foreleg_l", 0, "spark_coil")]),
        ],
        &[spec(
            "pangol",
            Side::Enemy,
            vec![
                ("arm_l", 0, "ember_spitter"),
                ("arm_r", 0, "ember_spitter"),
                ("back", 0, "basalt_carapace"),
                ("tail", 0, "shield_membrane"),
            ],
        )],
        RiderMods::neutral(),
        5,
    )
    .unwrap();
    battle.rider.mounted_on = None;

    run_to_outcome(&mut battle, &data, 400.0);
    let Some(BattleOutcome::Victory(rewards)) = battle.outcome.clone() else {
        panic!("expected victory, got {:?}", battle.outcome);
    };
    assert!(
        !rewards.salvage.is_empty(),
        "an enemy stripped of four grafts should drop something"
    );
    assert_eq!(rewards.captured_species, vec!["pangol".to_owned()]);
}
