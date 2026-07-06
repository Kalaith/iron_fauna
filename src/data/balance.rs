//! Balance tuning loaded from `assets/data/balance.json`.
//!
//! Every combat/derivation coefficient lives here so tuning happens in data,
//! not code (`CODE_STANDARDS.md`: data-driven design is a hard rule).

use crate::data::species::{SizeClass, Temperament};
use serde::{Deserialize, Serialize};

/// A value scaled per creature size class.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SizeScaled {
    pub small: f32,
    pub medium: f32,
    pub large: f32,
    pub huge: f32,
}

impl SizeScaled {
    pub fn get(&self, size: SizeClass) -> f32 {
        match size {
            SizeClass::Small => self.small,
            SizeClass::Medium => self.medium,
            SizeClass::Large => self.large,
            SizeClass::Huge => self.huge,
        }
    }
}

/// A value scaled per temperament.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TemperamentScaled {
    pub gentle: f32,
    pub placid: f32,
    pub wary: f32,
    pub fierce: f32,
}

impl TemperamentScaled {
    pub fn get(&self, t: Temperament) -> f32 {
        match t {
            Temperament::Gentle => self.gentle,
            Temperament::Placid => self.placid,
            Temperament::Wary => self.wary,
            Temperament::Fierce => self.fierce,
        }
    }
}

/// Curves deriving battle stats from chassis stats (`creature.md` §2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeciesCurves {
    pub power_capacity_mult: f32,
    pub vigor_base: f32,
    pub vigor_per_power: f32,
    pub vigor_size_bonus: SizeScaled,
    pub vigor_regen_base: f32,
    pub vigor_regen_per_power: f32,
    pub strain_base: f32,
    pub strain_per_power: f32,
    pub temperament_strain_mult: TemperamentScaled,
    pub limb_hp_base: f32,
    pub limb_hp_per_power: f32,
    pub limb_hp_size_mult: SizeScaled,
    pub core_hp_base: f32,
    pub core_hp_per_power: f32,
    pub regrow_base: f32,
    pub regrow_per_power: f32,
    pub dodge_per_speed: f32,
    pub size_dodge_bonus: SizeScaled,
    /// Speeds below this pivot earn an accuracy bonus (stable chassis).
    pub accuracy_slow_pivot: f32,
    pub accuracy_slow_per_point: f32,
    pub move_speed_base: f32,
    pub move_speed_per_speed: f32,
    pub size_move_mult: SizeScaled,
    /// Bond level bonuses (per point): vigor % and strain-threshold %.
    pub bond_vigor_pct: f32,
    pub bond_strain_pct: f32,
}

/// Vigor economy knobs (`game_design.md` §4.2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VigorTuning {
    /// Extra vigor regen multiplier while ridden (the Boost, `combat.md` §3).
    pub ridden_regen_mult: f32,
    /// Vigor spent per limb HP regrown.
    pub regrow_cost_per_hp: f32,
    /// Reinforce action: vigor cost and core shield granted.
    pub reinforce_cost: f32,
    pub reinforce_shield: f32,
    pub reinforce_cooldown: f32,
    /// Max core shield as a fraction of core max HP.
    pub reinforce_shield_cap_frac: f32,
}

/// A value scaled per graftware weight class.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct WeightScaled {
    pub light: f32,
    pub medium: f32,
    pub heavy: f32,
}

impl WeightScaled {
    pub fn get(&self, w: crate::data::species::WeightClass) -> f32 {
        use crate::data::species::WeightClass;
        match w {
            WeightClass::Light => self.light,
            WeightClass::Medium => self.medium,
            WeightClass::Heavy => self.heavy,
        }
    }
}

/// Strain accumulation and consequences (`game_design.md` §4.3).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrainTuning {
    /// Passive strain/sec while ridden (the Boost pushes the creature).
    pub ridden_gain_per_sec: f32,
    /// Strain/sec per point of power-draw overdraw.
    pub overdraw_gain_per_point: f32,
    /// Strain added per weapon shot fired (scaled by weapon power draw).
    pub fire_gain_per_draw: f32,
    /// Strain/sec recovered while not ridden and not overdrawn.
    pub calm_decay_per_sec: f32,
    /// Accuracy penalty at 100% strain (linear from 0).
    pub accuracy_penalty_at_max: f32,
    /// Regrow-speed penalty at 100% strain (linear from 0).
    pub regrow_penalty_at_max: f32,
    /// Seconds a berserk episode lasts.
    pub berserk_duration: f32,
    /// At threshold: chance per second of berserk (vs graft rejection).
    pub berserk_chance: f32,
    /// Fraction of strain vented when a graft is rejected.
    pub rejection_relief_frac: f32,
}

/// General battle pacing (`combat.md`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BattleTuning {
    /// Accuracy multiplier applied to called shots (aimed at one mount/limb).
    pub called_shot_accuracy_mult: f32,
    pub base_accuracy: f32,
    /// Chance a destroyed mount's graftware drops as battlefield salvage.
    pub salvage_drop_chance: f32,
    /// Seconds the rider spends exposed mid-hop.
    pub hop_transit_time: f32,
    /// Damage multiplier against an exposed core.
    pub exposed_core_damage_mult: f32,
    /// Range bands in world units.
    pub melee_range: f32,
    pub short_range: f32,
    pub long_range: f32,
    /// Arena half-width in world units.
    pub arena_half_width: f32,
    /// Fraction of damage spilling to the limb when a mounted graft is hit.
    pub graft_spill_frac: f32,
    /// Natural (unarmed) attack: damage scale per point of creature power.
    pub natural_attack_per_power: f32,
    pub natural_attack_cooldown: f32,
    /// Element match multiplier for graftware on same-element chassis.
    pub element_synergy_mult: f32,
    /// Structural HP of a mounted graft when targeted directly, by weight.
    pub graft_hp: WeightScaled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceConfig {
    /// Field/party budget in slots (`combat.md` §2.1).
    pub party_slot_budget: u32,
    pub curves: SpeciesCurves,
    pub vigor: VigorTuning,
    pub strain: StrainTuning,
    pub battle: BattleTuning,
}

#[cfg(test)]
mod tests {
    use crate::data::GameData;

    #[test]
    fn balance_loads_with_sane_values() {
        let bal = GameData::load().unwrap().balance;
        assert_eq!(bal.party_slot_budget, 6);
        assert!(bal.vigor.ridden_regen_mult > 1.0);
        assert!(bal.battle.called_shot_accuracy_mult <= 1.0);
        assert!(bal.battle.melee_range < bal.battle.short_range);
        assert!(bal.battle.short_range < bal.battle.long_range);
        assert!(bal.battle.element_synergy_mult > 1.0);
    }
}
