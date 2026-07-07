//! Gestarium (bio-factory) definitions — the game's dungeons and the anchor
//! of the verdict system (`game_design.md` §7, §9).

use crate::data::settlement::DuelUnitDef;
use crate::data::world::MapDef;
use crate::model::rider::RiderUpgrade;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactoryDef {
    pub id: String,
    pub name: String,
    pub region: String,
    pub description: String,
    /// Floor maps, entrance first, heart floor last. Merged into the world
    /// map list at load time so traversal works unchanged.
    pub floors: Vec<MapDef>,
    /// The authored heart-guardian fight on the deepest floor.
    pub heart_guard: Vec<DuelUnitDef>,
    /// The rider's permanent upgrade for silencing this factory (§3).
    pub rider_upgrade: RiderUpgrade,
    /// Species this factory can gestate for you once Bound (§9 Bind).
    #[serde(default)]
    pub grows: Vec<String>,
    /// Scrip cost to grow a core here once Bound.
    #[serde(default)]
    pub grow_cost: i64,
}

#[cfg(test)]
mod tests {
    use crate::combat::unit::{BattleUnit, UnitSpec};
    use crate::combat::{Side, Stance};
    use crate::data::world::MapKind;
    use crate::data::GameData;

    #[test]
    fn factories_are_valid() {
        let data = GameData::load().unwrap();
        assert!(!data.factories.is_empty());

        for (_, f) in data.factories.iter() {
            assert!(
                data.world.region(&f.region).is_some(),
                "{}: bad region",
                f.id
            );
            assert!(!f.floors.is_empty(), "{}: no floors", f.id);
            assert!(!f.heart_guard.is_empty(), "{}: unguarded heart", f.id);
            assert!(f.grow_cost > 0, "{}: free cores", f.id);
            for species in &f.grows {
                assert!(
                    data.species.contains(species),
                    "{}: bad grow {}",
                    f.id,
                    species
                );
            }
            // Floors were merged into the world map list with factory kind.
            for floor in &f.floors {
                let merged = data
                    .world
                    .map(&floor.id)
                    .unwrap_or_else(|| panic!("{}: floor {} not merged", f.id, floor.id));
                assert_eq!(merged.kind, MapKind::Factory);
                assert_eq!(merged.factory_id.as_deref(), Some(f.id.as_str()));
            }
            // The deepest floor must actually contain a heart tile.
            let deepest = f.floors.last().unwrap();
            let has_heart = deepest.rows.iter().any(|r| r.contains('H'));
            assert!(has_heart, "{}: heartless deepest floor", f.id);
            // Heart guardians must assemble into legal units.
            for unit in &f.heart_guard {
                let spec = UnitSpec {
                    species_id: unit.species.clone(),
                    name: unit.name.clone().unwrap_or_else(|| f.name.clone()),
                    side: Side::Enemy,
                    creature_id: None,
                    bond: 0.0,
                    stance: Stance::Aggressive,
                    grafts: unit
                        .grafts
                        .iter()
                        .map(|g| (g.limb.clone(), g.slot, g.graft.clone(), None))
                        .collect(),
                };
                BattleUnit::build(&spec, &data, 0.0)
                    .unwrap_or_else(|err| panic!("{}: illegal guard: {}", f.id, err));
            }
        }
    }
}
