//! Procedural war-unit outfitting: factory-born creatures come pre-armed
//! (`game_design.md` §7). Deterministic given the battle seed.

use crate::data::graftware::GraftKind;
use crate::data::species::SpeciesDef;
use crate::data::GameData;
use crate::util::Rng;

/// Fills a fraction of a species' mounts with tier-appropriate graftware.
/// Returns `(limb_id, slot, graft_def_id, None)` tuples ready for a UnitSpec.
pub fn war_unit_grafts(
    species: &SpeciesDef,
    data: &GameData,
    max_tier: u32,
    rng: &mut Rng,
) -> Vec<(String, usize, String, Option<u64>)> {
    // Candidate pool sorted by id — registry iteration order is not stable,
    // and battle generation must be deterministic.
    let mut catalog: Vec<_> = data
        .graftware
        .iter()
        .filter(|(_, g)| g.tier <= max_tier && g.min_power <= species.power)
        .collect();
    catalog.sort_by_key(|(id, _)| (*id).clone());

    let mut grafts = Vec::new();
    for limb in &species.limbs {
        for (slot, class) in limb.mounts.iter().enumerate() {
            if !rng.chance(0.7) {
                continue;
            }
            // Weapons dominate a war-unit's kit; armor and utility pad it out.
            let roll = rng.next_f32();
            let want = if roll < 0.65 {
                GraftKind::Weapon
            } else if roll < 0.85 {
                GraftKind::Armor
            } else {
                GraftKind::Utility
            };
            let fits: Vec<_> = catalog
                .iter()
                .filter(|(_, g)| g.weight <= *class && g.kind == want)
                .collect();
            let pool = if fits.is_empty() {
                // Fall back to anything that fits the mount.
                catalog
                    .iter()
                    .filter(|(_, g)| g.weight <= *class)
                    .collect::<Vec<_>>()
            } else {
                fits
            };
            if pool.is_empty() {
                continue;
            }
            let (id, _) = pool[rng.below(pool.len())];
            grafts.push((limb.id.clone(), slot, (*id).clone(), None));
        }
    }
    grafts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::unit::{BattleUnit, UnitSpec};
    use crate::combat::{Side, Stance};
    use crate::data::GameData;

    #[test]
    fn war_units_are_always_legal_and_deterministic() {
        let data = GameData::load().unwrap();
        for (_, species) in data.species.iter() {
            let mut rng_a = Rng::new(31 + species.power as u64);
            let mut rng_b = Rng::new(31 + species.power as u64);
            let grafts = war_unit_grafts(species, &data, species.tier.max(2), &mut rng_a);
            let again = war_unit_grafts(species, &data, species.tier.max(2), &mut rng_b);
            assert_eq!(grafts, again, "{}: nondeterministic outfit", species.id);

            let spec = UnitSpec {
                species_id: species.id.clone(),
                name: species.name.clone(),
                side: Side::Enemy,
                creature_id: None,
                bond: 0.0,
                stance: Stance::Aggressive,
                grafts,
            };
            BattleUnit::build(&spec, &data)
                .unwrap_or_else(|err| panic!("{}: illegal war unit: {}", species.id, err));
        }
    }
}
