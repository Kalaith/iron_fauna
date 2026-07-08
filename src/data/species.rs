//! Species (chassis) definitions — see `creature.md`.
//!
//! A species is a chassis: Power, Size, Speed, Limb Count, Element. All
//! combat-facing numbers are derived from these via `BalanceConfig` curves so
//! balance lives in `assets/data/balance.json`, not in code.

use crate::data::balance::BalanceConfig;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Element {
    BioElectric,
    Plant,
    Rock,
    Fire,
    Water,
    Poison,
}

impl Element {
    pub fn display_name(self) -> &'static str {
        match self {
            Element::BioElectric => "Bio-Electric",
            Element::Plant => "Plant",
            Element::Rock => "Rock",
            Element::Fire => "Fire",
            Element::Water => "Water",
            Element::Poison => "Poison",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SizeClass {
    Small,
    Medium,
    Large,
    Huge,
}

impl SizeClass {
    /// Party/battle field budget cost (`combat.md` §2.1).
    pub fn slot_cost(self) -> u32 {
        match self {
            SizeClass::Small => 1,
            SizeClass::Medium => 2,
            SizeClass::Large => 3,
            SizeClass::Huge => 4,
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            SizeClass::Small => "Small",
            SizeClass::Medium => "Medium",
            SizeClass::Large => "Large",
            SizeClass::Huge => "Huge",
        }
    }
}

/// Strain-tolerance personality (`game_design.md` §4.3): gentler creatures
/// have the lowest strain tolerance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Temperament {
    Gentle,
    Placid,
    Wary,
    Fierce,
}

impl Temperament {
    pub fn display_name(self) -> &'static str {
        match self {
            Temperament::Gentle => "Gentle",
            Temperament::Placid => "Placid",
            Temperament::Wary => "Wary",
            Temperament::Fierce => "Fierce",
        }
    }
}

/// Limb-count archetype (`creature.md` §2.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LimbArchetype {
    Flier,
    Standard,
    Utility,
    Heavy,
}

impl LimbArchetype {
    pub fn display_name(self) -> &'static str {
        match self {
            LimbArchetype::Flier => "Flier",
            LimbArchetype::Standard => "Standard",
            LimbArchetype::Utility => "Utility",
            LimbArchetype::Heavy => "Heavy",
        }
    }
}

/// Graftware weight class a mount point accepts (also used by graftware defs).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum WeightClass {
    Light,
    Medium,
    Heavy,
}

impl WeightClass {
    pub fn display_name(self) -> &'static str {
        match self {
            WeightClass::Light => "Light",
            WeightClass::Medium => "Medium",
            WeightClass::Heavy => "Heavy",
        }
    }
}

/// Where a limb sits on the sprite — drives called-shot directional mapping
/// (`combat.md` §3.1: up = head/back, down = legs, left/right = arms).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LimbRegion {
    Head,
    Back,
    ArmLeft,
    ArmRight,
    Legs,
    Tail,
}

impl LimbRegion {
    pub fn display_name(self) -> &'static str {
        match self {
            LimbRegion::Head => "Head",
            LimbRegion::Back => "Back",
            LimbRegion::ArmLeft => "Left Arm",
            LimbRegion::ArmRight => "Right Arm",
            LimbRegion::Legs => "Legs",
            LimbRegion::Tail => "Tail",
        }
    }
}

/// One war-body limb: a targetable body segment that can host graftware
/// mounts. Limbs regrow while the core lives; mounted graftware does not.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimbDef {
    pub id: String,
    pub name: String,
    pub region: LimbRegion,
    /// Mount points on this limb (0..=2), by accepted weight class.
    #[serde(default)]
    pub mounts: Vec<WeightClass>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeciesDef {
    pub id: String,
    pub name: String,
    pub description: String,
    pub element: Element,
    pub size: SizeClass,
    pub temperament: Temperament,
    pub archetype: LimbArchetype,
    /// Biological capacity, 0-100 (`creature.md` §2.1).
    pub power: u32,
    /// 0-100. Fast = dodge + battle movement + overworld pace.
    pub speed: u32,
    #[serde(default)]
    pub natural_flight: bool,
    /// Flat innate shell armor (subtracted from limb hits).
    #[serde(default)]
    pub innate_armor: u32,
    pub limbs: Vec<LimbDef>,
    /// Content tier 1-4: gates wild placement and factory depth.
    pub tier: u32,
    /// Biome tags for encounter tables (e.g. "meadow", "marsh", "caldera").
    #[serde(default)]
    pub biomes: Vec<String>,
}

/// Battle-facing numbers derived from chassis stats through balance curves.
#[derive(Debug, Clone, Copy)]
pub struct DerivedStats {
    pub power_capacity: f32,
    pub vigor_max: f32,
    pub vigor_regen: f32,
    pub strain_threshold: f32,
    pub limb_hp: f32,
    pub core_hp: f32,
    pub regrow_hp_per_sec: f32,
    /// 0-1 chance to fully evade an incoming hit.
    pub dodge: f32,
    /// Additive accuracy bonus for slow, stable chassis.
    pub accuracy_bonus: f32,
}

impl SpeciesDef {
    pub fn derived(&self, bal: &BalanceConfig) -> DerivedStats {
        let c = &bal.curves;
        let power = self.power as f32;
        let speed = self.speed as f32;
        let size_i = self.size;
        DerivedStats {
            power_capacity: power * c.power_capacity_mult,
            vigor_max: c.vigor_base + power * c.vigor_per_power + c.vigor_size_bonus.get(size_i),
            vigor_regen: c.vigor_regen_base + power * c.vigor_regen_per_power,
            strain_threshold: (c.strain_base + power * c.strain_per_power)
                * c.temperament_strain_mult.get(self.temperament),
            limb_hp: (c.limb_hp_base + power * c.limb_hp_per_power)
                * c.limb_hp_size_mult.get(size_i),
            core_hp: c.core_hp_base + power * c.core_hp_per_power,
            regrow_hp_per_sec: c.regrow_base + power * c.regrow_per_power,
            dodge: (speed * c.dodge_per_speed + c.size_dodge_bonus.get(size_i)).clamp(0.0, 0.2),
            accuracy_bonus: ((c.accuracy_slow_pivot - speed).max(0.0)) * c.accuracy_slow_per_point,
        }
    }

    pub fn mount_count(&self) -> usize {
        self.limbs.iter().map(|l| l.mounts.len()).sum()
    }

    pub fn limb(&self, limb_id: &str) -> Option<&LimbDef> {
        self.limbs.iter().find(|l| l.id == limb_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::GameData;

    #[test]
    fn roster_is_valid() {
        let data = GameData::load().unwrap();
        let bal = &data.balance;
        assert!(
            data.species.len() >= 4,
            "expected at least seed roster, found {}",
            data.species.len()
        );

        for (_, sp) in data.species.iter() {
            assert!(sp.power <= 100, "{}: power out of range", sp.id);
            assert!(sp.speed <= 100, "{}: speed out of range", sp.id);
            assert!((1..=4).contains(&sp.tier), "{}: bad tier", sp.id);
            assert!(!sp.limbs.is_empty(), "{}: no limbs", sp.id);
            let mut limb_ids: Vec<&str> = sp.limbs.iter().map(|l| l.id.as_str()).collect();
            limb_ids.sort_unstable();
            limb_ids.dedup();
            assert_eq!(
                limb_ids.len(),
                sp.limbs.len(),
                "{}: duplicate limb ids",
                sp.id
            );
            for limb in &sp.limbs {
                assert!(
                    limb.mounts.len() <= 2,
                    "{}: limb {} has >2 mounts",
                    sp.id,
                    limb.id
                );
            }
            // Fliers trade mounts for flight (creature.md §2.4).
            if sp.archetype == LimbArchetype::Flier {
                assert!(sp.natural_flight, "{}: flier without flight", sp.id);
                assert!(
                    sp.mount_count() <= 2,
                    "{}: flier with too many mounts",
                    sp.id
                );
            }

            let d = sp.derived(bal);
            assert!(d.vigor_max > 0.0 && d.core_hp > 0.0 && d.limb_hp > 0.0);
            assert!(d.strain_threshold > 0.0);
            assert!(d.power_capacity > 0.0);
        }
    }

    #[test]
    fn gentle_temperament_lowers_strain_threshold() {
        let data = GameData::load().unwrap();
        let mult = &data.balance.curves.temperament_strain_mult;
        assert!(mult.gentle < mult.fierce);
    }
}
