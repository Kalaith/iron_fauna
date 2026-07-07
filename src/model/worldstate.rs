//! Persistent world state: the sum of the player's factory verdicts
//! (`game_design.md` §9). Regions remember what you decided.

use crate::data::GameData;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The region-defining decision at each Gestarium.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Verdict {
    /// Shut it down forever: safe, but the region stays dead.
    Purge,
    /// Restore its benign function: the region revives — and can relapse.
    Reseed,
    /// Claim it: grow your own cores with the old civilization's machine.
    Bind,
}

impl Verdict {
    pub fn display_name(self) -> &'static str {
        match self {
            Verdict::Purge => "Purged",
            Verdict::Reseed => "Reseeded",
            Verdict::Bind => "Bound",
        }
    }
}

/// How a region currently reads, derived from its factory's state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionMood {
    /// Factory alive and hostile: the default danger.
    Threatened,
    /// Purged: graveyard peace.
    Dead,
    /// Reseeded: blooming, and quietly at risk.
    Reviving,
    /// Bound: the machine answers to you now.
    Claimed,
    /// A reseeded region that slid back — someone is grafting again (§9.1).
    Relapsed,
}

impl RegionMood {
    pub fn display_name(self) -> &'static str {
        match self {
            RegionMood::Threatened => "Threatened",
            RegionMood::Dead => "Dead",
            RegionMood::Reviving => "Reviving",
            RegionMood::Claimed => "Claimed",
            RegionMood::Relapsed => "Relapsed",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FactoryState {
    pub heart_defeated: bool,
    pub verdict: Option<Verdict>,
    /// Grows with time after a Reseed left untended — relapse fuel (§9.1).
    #[serde(default)]
    pub relapse_risk: f32,
    /// Stewardship investment lowers relapse risk at ongoing cost (§9.1).
    #[serde(default)]
    pub invested: bool,
    /// The region slid back into weaponizing — confront it to end it.
    #[serde(default)]
    pub relapsed: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorldState {
    pub factories: HashMap<String, FactoryState>,
}

impl WorldState {
    pub fn factory(&self, id: &str) -> FactoryState {
        self.factories.get(id).cloned().unwrap_or_default()
    }

    pub fn factory_mut(&mut self, id: &str) -> &mut FactoryState {
        self.factories.entry(id.to_owned()).or_default()
    }

    pub fn verdict(&self, id: &str) -> Option<Verdict> {
        self.factory(id).verdict
    }

    /// A region's mood follows its anchoring factory's state.
    pub fn region_mood(&self, data: &GameData, region_id: &str) -> RegionMood {
        let Some(region) = data.world.region(region_id) else {
            return RegionMood::Threatened;
        };
        let state = self.factory(&region.gestarium_id);
        match state.verdict {
            None => RegionMood::Threatened,
            Some(Verdict::Purge) => RegionMood::Dead,
            Some(Verdict::Reseed) if state.relapsed => RegionMood::Relapsed,
            Some(Verdict::Reseed) => RegionMood::Reviving,
            Some(Verdict::Bind) => RegionMood::Claimed,
        }
    }

    /// Wild encounter richness under each mood: purged ground goes quiet,
    /// reseeded ground blooms.
    pub fn encounter_rate_mult(mood: RegionMood) -> f32 {
        match mood {
            RegionMood::Threatened => 1.0,
            RegionMood::Dead => 0.25,
            RegionMood::Reviving => 1.4,
            RegionMood::Claimed => 1.0,
            RegionMood::Relapsed => 1.2,
        }
    }

    /// Factory floors go dormant once any verdict is passed — the vats stop.
    pub fn factory_active(&self, id: &str) -> bool {
        self.verdict(id).is_none()
    }

    /// How many of the world's Gestaria have received a verdict.
    pub fn verdicts_passed(&self, data: &GameData) -> usize {
        data.factories
            .iter()
            .filter(|(id, _)| self.verdict(id).is_some())
            .count()
    }

    /// True once every Gestarium in the world has been judged — the endgame
    /// condition (`game_design.md` §9.2).
    pub fn all_judged(&self, data: &GameData) -> bool {
        !data.factories.is_empty() && self.verdicts_passed(data) == data.factories.len()
    }

    /// Tally of verdicts by kind, for the closing reflection.
    pub fn verdict_tally(&self, data: &GameData) -> (usize, usize, usize) {
        let mut purge = 0;
        let mut reseed = 0;
        let mut bind = 0;
        for (id, _) in data.factories.iter() {
            match self.verdict(id) {
                Some(Verdict::Purge) => purge += 1,
                Some(Verdict::Reseed) => reseed += 1,
                Some(Verdict::Bind) => bind += 1,
                None => {}
            }
        }
        (purge, reseed, bind)
    }

    /// Advances relapse pressure by one overworld step (§9.1: revive-and-
    /// abandon is high risk; funding the watch slows the slide). Returns the
    /// id of a factory that just tipped into relapse, if any.
    pub fn tick_relapse(&mut self, per_step: f32, invested_mult: f32) -> Option<String> {
        for (id, state) in self.factories.iter_mut() {
            if state.verdict == Some(Verdict::Reseed) && !state.relapsed {
                let rate = if state.invested {
                    per_step * invested_mult
                } else {
                    per_step
                };
                state.relapse_risk += rate;
                if state.relapse_risk >= 1.0 {
                    state.relapsed = true;
                    state.relapse_risk = 1.0;
                    return Some(id.clone());
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::GameData;

    #[test]
    fn verdicts_reshape_the_region() {
        let data = GameData::load().unwrap();
        let mut world = WorldState::default();
        assert_eq!(
            world.region_mood(&data, "verdant_hollow"),
            RegionMood::Threatened
        );
        assert!(world.factory_active("the_cradle"));

        world.factory_mut("the_cradle").heart_defeated = true;
        world.factory_mut("the_cradle").verdict = Some(Verdict::Purge);
        assert_eq!(world.region_mood(&data, "verdant_hollow"), RegionMood::Dead);
        assert!(!world.factory_active("the_cradle"));

        world.factory_mut("the_cradle").verdict = Some(Verdict::Reseed);
        assert_eq!(
            world.region_mood(&data, "verdant_hollow"),
            RegionMood::Reviving
        );
        assert!(
            WorldState::encounter_rate_mult(RegionMood::Reviving)
                > WorldState::encounter_rate_mult(RegionMood::Dead)
        );
    }

    #[test]
    fn untended_reseed_relapses_and_investment_slows_it() {
        let data = GameData::load().unwrap();
        let mut world = WorldState::default();
        world.factory_mut("the_cradle").verdict = Some(Verdict::Reseed);

        // Untended: ticks accumulate to relapse.
        let mut tipped = None;
        for _ in 0..5000 {
            if let Some(id) = world.tick_relapse(0.001, 0.15) {
                tipped = Some(id);
                break;
            }
        }
        assert_eq!(tipped.as_deref(), Some("the_cradle"));
        assert_eq!(
            world.region_mood(&data, "verdant_hollow"),
            RegionMood::Relapsed
        );

        // Confronted: back to reviving.
        let state = world.factory_mut("the_cradle");
        state.relapsed = false;
        state.relapse_risk = 0.0;
        assert_eq!(
            world.region_mood(&data, "verdant_hollow"),
            RegionMood::Reviving
        );

        // Invested: the same number of ticks doesn't tip.
        let mut watched = WorldState::default();
        watched.factory_mut("the_cradle").verdict = Some(Verdict::Reseed);
        watched.factory_mut("the_cradle").invested = true;
        for _ in 0..5000 {
            assert!(watched.tick_relapse(0.001, 0.15).is_none());
        }
        assert!(watched.factory("the_cradle").relapse_risk < 1.0);
    }
}
