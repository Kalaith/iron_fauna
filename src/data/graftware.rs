//! Graftware definitions — semi-living weapons, armor, and utility organs
//! grafted onto war-body limbs (`game_design.md` §3, `creature.md` §5).

use crate::data::species::{Element, WeightClass};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GraftKind {
    Weapon,
    Armor,
    Utility,
}

impl GraftKind {
    pub fn display_name(self) -> &'static str {
        match self {
            GraftKind::Weapon => "Weapon",
            GraftKind::Armor => "Armor",
            GraftKind::Utility => "Utility",
        }
    }
}

/// Continuous or triggered effect for utility graftware.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum GraftEffect {
    /// Triggered: heal a friendly unit's most damaged limb (or core).
    Heal {
        amount: f32,
        cooldown: f32,
        vigor_cost: f32,
    },
    /// Triggered: shield own core.
    ShieldCore {
        amount: f32,
        cooldown: f32,
        vigor_cost: f32,
    },
    /// Passive: extra vigor regen per second.
    VigorTap { per_sec: f32 },
    /// Passive: vents strain per second.
    StrainVent { per_sec: f32 },
    /// Passive: accuracy bonus for the whole unit.
    SensorArray { accuracy_bonus: f32 },
    /// Passive: limb regrowth speed multiplier.
    GrowthGland { regrow_mult: f32 },
}

impl GraftEffect {
    /// Whether this effect is player-triggered (vs. an always-on passive).
    pub fn is_triggered(self) -> bool {
        matches!(
            self,
            GraftEffect::Heal { .. } | GraftEffect::ShieldCore { .. }
        )
    }
}

/// What the Boost does with this graft equipped while ridden
/// (`combat.md` §3.2 — Boost depends on creature and equipped graftware).
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum BoostEffect {
    /// Weapon hits arc to additional nearby enemies.
    ChainArc { extra_targets: u32, falloff: f32 },
    /// Weapon fires additional projectiles per shot.
    Barrage { extra_shots: u32 },
    /// Triggered utility effects are amplified.
    Amplify { mult: f32 },
    /// Armor also grants damage reduction to the core.
    Bulwark { core_damage_mult: f32 },
    /// Limb regrowth is accelerated while ridden.
    Overgrowth { regrow_mult: f32 },
    /// Weapon hits apply a damage-over-time (venom/burn).
    Corrode { dps: f32, duration: f32 },
    /// No special boost behavior (still gets ridden vigor regen).
    #[default]
    Plain,
}

impl BoostEffect {
    /// A one-line, player-facing description of what riding unlocks with this
    /// graft equipped — None for `Plain` (nothing to advertise).
    pub fn describe(self) -> Option<String> {
        Some(match self {
            BoostEffect::ChainArc { extra_targets, .. } => {
                format!("its shots arc to {} more foe(s)", extra_targets)
            }
            BoostEffect::Barrage { extra_shots } => {
                format!("it fires {} extra shot(s) per volley", extra_shots)
            }
            BoostEffect::Amplify { mult } => {
                format!("its utility bursts hit {:.0}% harder", mult * 100.0)
            }
            BoostEffect::Bulwark { core_damage_mult } => format!(
                "its plating shields the core ({:.0}% incoming)",
                core_damage_mult * 100.0
            ),
            BoostEffect::Overgrowth { regrow_mult } => {
                format!("its limbs regrow {:.0}% faster", regrow_mult * 100.0)
            }
            BoostEffect::Corrode { dps, duration } => {
                format!("its hits corrode ({:.0}/s for {:.0}s)", dps, duration)
            }
            BoostEffect::Plain => return None,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraftwareDef {
    pub id: String,
    pub name: String,
    pub description: String,
    pub kind: GraftKind,
    pub weight: WeightClass,
    /// Sustained power-capacity load while equipped (`creature.md` §5).
    pub power_draw: u32,
    /// Minimum creature Power stat required to wield at all.
    #[serde(default)]
    pub min_power: u32,
    /// Element affinity — synergizes on matching-element chassis; None = neutral.
    #[serde(default)]
    pub element: Option<Element>,
    /// Content tier 1-4, gating factory depth / shop availability.
    pub tier: u32,
    /// Base currency value (repair cost derives from this).
    pub value: i64,

    // Weapon fields (ignored for pure armor/utility).
    #[serde(default)]
    pub damage: f32,
    #[serde(default)]
    pub cooldown: f32,
    #[serde(default)]
    pub vigor_cost: f32,

    // Armor field: flat damage reduction added to the limb it's mounted on
    // (and reduces spill-through to the limb).
    #[serde(default)]
    pub armor: f32,
    /// Armor plating has its own HP pool that ablates before the limb.
    #[serde(default)]
    pub plating_hp: f32,

    #[serde(default)]
    pub effect: Option<GraftEffect>,
    #[serde(default)]
    pub boost: BoostEffect,
}

impl GraftwareDef {
    pub fn is_weapon(&self) -> bool {
        self.kind == GraftKind::Weapon
    }

    pub fn repair_cost(&self) -> i64 {
        (self.value / 2).max(5)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::GameData;

    #[test]
    fn catalog_is_valid() {
        let data = GameData::load().unwrap();
        assert!(
            data.graftware.len() >= 6,
            "expected at least seed catalog, found {}",
            data.graftware.len()
        );

        for (_, g) in data.graftware.iter() {
            assert!((1..=4).contains(&g.tier), "{}: bad tier", g.id);
            assert!(g.power_draw > 0, "{}: free graftware", g.id);
            assert!(g.value > 0, "{}: no value", g.id);
            match g.kind {
                GraftKind::Weapon => {
                    assert!(g.damage > 0.0, "{}: weapon without damage", g.id);
                    assert!(g.cooldown > 0.0, "{}: weapon without cooldown", g.id);
                    assert!(g.vigor_cost > 0.0, "{}: weapon fires for free", g.id);
                }
                GraftKind::Armor => {
                    assert!(
                        g.armor > 0.0 || g.plating_hp > 0.0,
                        "{}: armor without protection",
                        g.id
                    );
                }
                GraftKind::Utility => {
                    assert!(g.effect.is_some(), "{}: utility without effect", g.id);
                }
            }
        }
    }

    #[test]
    fn heavier_tiers_draw_more_power_on_average() {
        let data = GameData::load().unwrap();
        let avg_draw = |w: WeightClass| {
            let (sum, n) = data
                .graftware
                .iter()
                .filter(|(_, g)| g.weight == w)
                .fold((0u32, 0u32), |(s, n), (_, g)| (s + g.power_draw, n + 1));
            if n == 0 {
                0.0
            } else {
                sum as f32 / n as f32
            }
        };
        let light = avg_draw(WeightClass::Light);
        let heavy = avg_draw(WeightClass::Heavy);
        if light > 0.0 && heavy > 0.0 {
            assert!(heavy > light, "heavy grafts should out-draw light ones");
        }
    }
}
