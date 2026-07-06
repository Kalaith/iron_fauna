//! The rider's own progression — one permanent upgrade per Gestarium
//! defeated (`game_design.md` §3), separate from creature/gear power.

use serde::{Deserialize, Serialize};

/// The six rider upgrades, one earned per factory heart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RiderUpgrade {
    /// +10% called-shot accuracy.
    SteadyHands,
    /// -30% rider-hop transit time.
    SwiftCrossing,
    /// -25% strain gain on the ridden creature.
    Beastwhisper,
    /// +30% limb regrowth speed while ridden.
    FieldMedic,
    /// +20% battlefield salvage drop chance.
    Salvager,
    /// +25% reinforce shield strength.
    Ironheart,
}

impl RiderUpgrade {
    pub const ALL: [RiderUpgrade; 6] = [
        RiderUpgrade::SteadyHands,
        RiderUpgrade::SwiftCrossing,
        RiderUpgrade::Beastwhisper,
        RiderUpgrade::FieldMedic,
        RiderUpgrade::Salvager,
        RiderUpgrade::Ironheart,
    ];

    pub fn display_name(self) -> &'static str {
        match self {
            RiderUpgrade::SteadyHands => "Steady Hands",
            RiderUpgrade::SwiftCrossing => "Swift Crossing",
            RiderUpgrade::Beastwhisper => "Beastwhisper",
            RiderUpgrade::FieldMedic => "Field Medic",
            RiderUpgrade::Salvager => "Salvager",
            RiderUpgrade::Ironheart => "Ironheart",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            RiderUpgrade::SteadyHands => "Called shots are 10% more accurate.",
            RiderUpgrade::SwiftCrossing => "Rider hops cross 30% faster.",
            RiderUpgrade::Beastwhisper => "Your mount gains strain 25% slower.",
            RiderUpgrade::FieldMedic => "Your mount regrows limbs 30% faster.",
            RiderUpgrade::Salvager => "20% more graftware survives as salvage.",
            RiderUpgrade::Ironheart => "Reinforcing shields the core 25% harder.",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rider {
    pub name: String,
    pub upgrades: Vec<RiderUpgrade>,
    /// Practice-duel ladder standing per settlement ring (ring id → rank).
    #[serde(default)]
    pub duel_ranks: std::collections::HashMap<String, u32>,
}

impl Rider {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            upgrades: Vec::new(),
            duel_ranks: Default::default(),
        }
    }

    pub fn has(&self, upgrade: RiderUpgrade) -> bool {
        self.upgrades.contains(&upgrade)
    }

    pub fn called_shot_accuracy_mult(&self) -> f32 {
        if self.has(RiderUpgrade::SteadyHands) {
            1.10
        } else {
            1.0
        }
    }

    pub fn hop_time_mult(&self) -> f32 {
        if self.has(RiderUpgrade::SwiftCrossing) {
            0.70
        } else {
            1.0
        }
    }

    pub fn strain_gain_mult(&self) -> f32 {
        if self.has(RiderUpgrade::Beastwhisper) {
            0.75
        } else {
            1.0
        }
    }

    pub fn ridden_regrow_mult(&self) -> f32 {
        if self.has(RiderUpgrade::FieldMedic) {
            1.30
        } else {
            1.0
        }
    }

    pub fn salvage_chance_bonus(&self) -> f32 {
        if self.has(RiderUpgrade::Salvager) {
            0.20
        } else {
            0.0
        }
    }

    pub fn reinforce_shield_mult(&self) -> f32 {
        if self.has(RiderUpgrade::Ironheart) {
            1.25
        } else {
            1.0
        }
    }
}
