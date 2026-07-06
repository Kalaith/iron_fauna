//! Semi-real-time battle engine (`combat.md`): continuous clock, cooldown
//! actions, Vigor economy, limb strip / core crack, rider possession.
//!
//! The engine is pause-agnostic: Wait/Active pacing is the game layer's job —
//! it simply stops calling `Battle::update` while paused.

pub mod actions;
pub mod ai;
pub mod engine;
pub mod events;
pub mod resolve;
pub mod unit;

#[cfg(test)]
mod sim_tests;

use crate::model::rider::Rider;
use serde::{Deserialize, Serialize};

pub type UnitId = usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Side {
    Player,
    Enemy,
}

impl Side {
    pub fn opponent(self) -> Side {
        match self {
            Side::Player => Side::Enemy,
            Side::Enemy => Side::Player,
        }
    }
}

/// Standing orders for autonomous creatures (`combat.md` §4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Stance {
    Aggressive,
    Defensive,
}

impl Stance {
    pub fn display_name(self) -> &'static str {
        match self {
            Stance::Aggressive => "Aggressive",
            Stance::Defensive => "Defensive",
        }
    }
}

/// The three combat contexts (`game_design.md` §5). Same engine; the context
/// only changes the fiction and how victory resolves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BattleContext {
    WildSubdue,
    FactoryDismantle,
    Duel,
}

/// A called shot's specific target on an enemy (`combat.md` §3.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalledTarget {
    /// Sever this limb (index into unit limbs).
    Limb(usize),
    /// Silence this mounted graft directly (index into unit mounts).
    Mount(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeaponRef {
    /// Natural melee (claws, tail) — always available, unarmed creatures' only option.
    Natural,
    /// A mounted weapon graft (index into unit mounts).
    Mount(usize),
}

/// Commands the player can issue — to the ridden creature, or party-wide.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlayerCommand {
    Fire {
        mount: usize,
        target: UnitId,
        called: Option<CalledTarget>,
    },
    NaturalAttack {
        target: UnitId,
        called: Option<CalledTarget>,
    },
    TriggerUtility {
        mount: usize,
        ally: Option<UnitId>,
    },
    Regrow {
        limb: usize,
    },
    Reinforce,
    Move {
        intent: f32,
    },
    SetStance {
        unit: UnitId,
        stance: Stance,
    },
    BeginHop {
        to: UnitId,
    },
}

/// Rider bonuses snapshotted into the battle (from `model::rider`).
#[derive(Debug, Clone, Copy)]
pub struct RiderMods {
    pub called_shot_mult: f32,
    pub hop_time_mult: f32,
    pub strain_gain_mult: f32,
    pub ridden_regrow_mult: f32,
    pub salvage_bonus: f32,
    pub reinforce_mult: f32,
}

impl RiderMods {
    pub fn from_rider(rider: &Rider) -> Self {
        Self {
            called_shot_mult: rider.called_shot_accuracy_mult(),
            hop_time_mult: rider.hop_time_mult(),
            strain_gain_mult: rider.strain_gain_mult(),
            ridden_regrow_mult: rider.ridden_regrow_mult(),
            salvage_bonus: rider.salvage_chance_bonus(),
            reinforce_mult: rider.reinforce_shield_mult(),
        }
    }

    #[cfg(test)]
    pub fn neutral() -> Self {
        Self {
            called_shot_mult: 1.0,
            hop_time_mult: 1.0,
            strain_gain_mult: 1.0,
            ridden_regrow_mult: 1.0,
            salvage_bonus: 0.0,
            reinforce_mult: 1.0,
        }
    }
}

/// What a won battle yields, before profile resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BattleRewards {
    /// Species ids of cracked cores — captures (wild) or freed cores (factory).
    pub captured_species: Vec<String>,
    /// Graftware def ids recovered from the battlefield.
    pub salvage: Vec<String>,
    pub scrip: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BattleOutcome {
    Victory(BattleRewards),
    /// Every fielded core cracked — you flee and escape (`combat.md` §7.1).
    Fled,
}

/// The rider's live position in the fight.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RiderState {
    pub mounted_on: Option<UnitId>,
    /// In-transit hop: (destination, seconds remaining).
    pub hop: Option<(UnitId, f32)>,
}

impl RiderState {
    /// Exposed: on the field with no mount and no hop underway
    /// (`combat.md` §7 — the mount was cracked under you).
    pub fn exposed(&self) -> bool {
        self.mounted_on.is_none() && self.hop.is_none()
    }
}
