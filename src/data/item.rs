//! Consumable items — potions used in combat, ammunition loaded into weapons,
//! and field kits used on the road. Distinct from graftware (which is mounted
//! permanently); consumables are spent on use.

use serde::{Deserialize, Serialize};

/// What using a consumable does. Potions resolve instantly on the ridden unit;
/// ammo is loaded into a weapon (a reload that takes a turn); field kits are
/// used out of combat.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ConsumableEffect {
    /// Heal the ridden unit's most-wounded limb.
    MendLimb { amount: f32 },
    /// Restore Vigor.
    RestoreVigor { amount: f32 },
    /// Vent accumulated Strain.
    VentStrain { amount: f32 },
    /// Shield the core.
    ShieldCore { amount: f32 },
    /// Repair one damaged graft in the field (out of combat).
    FieldRepair,
    /// Load a magazine into a weapon; shots gain `damage_mult` and an optional
    /// burn until the magazine empties.
    Ammo {
        magazine: u32,
        damage_mult: f32,
        #[serde(default)]
        burn_dps: f32,
        #[serde(default)]
        burn_secs: f32,
    },
}

impl ConsumableEffect {
    pub fn is_potion(self) -> bool {
        matches!(
            self,
            ConsumableEffect::MendLimb { .. }
                | ConsumableEffect::RestoreVigor { .. }
                | ConsumableEffect::VentStrain { .. }
                | ConsumableEffect::ShieldCore { .. }
        )
    }

    pub fn is_ammo(self) -> bool {
        matches!(self, ConsumableEffect::Ammo { .. })
    }

    pub fn is_field_repair(self) -> bool {
        matches!(self, ConsumableEffect::FieldRepair)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsumableDef {
    pub id: String,
    pub name: String,
    pub description: String,
    pub effect: ConsumableEffect,
    /// Shop value (buy price); sale/derived costs come off this.
    pub value: i64,
}

impl ConsumableDef {
    /// Short category tag for the UI.
    pub fn category(&self) -> &'static str {
        if self.effect.is_ammo() {
            "Ammo"
        } else if self.effect.is_field_repair() {
            "Field Kit"
        } else {
            "Potion"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::GameData;

    #[test]
    fn consumables_are_valid() {
        let data = GameData::load().unwrap();
        assert!(!data.items.is_empty(), "no consumables authored");
        for (_, it) in data.items.iter() {
            assert!(it.value > 0, "{}: no value", it.id);
            if let ConsumableEffect::Ammo {
                magazine,
                damage_mult,
                ..
            } = it.effect
            {
                assert!(magazine > 0, "{}: empty magazine", it.id);
                assert!(damage_mult > 0.0, "{}: zero damage", it.id);
            }
        }
    }
}
