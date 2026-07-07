//! Runtime domain model — engine-agnostic player and world state.

pub mod creature;
pub mod duel;
pub mod inventory;
pub mod party;
pub mod quest;
pub mod rider;
pub mod story;
pub mod warunit;
pub mod worldstate;

use crate::data::GameData;
use creature::{CreatureInstance, CreatureOrigin};
use inventory::{GraftCondition, Inventory};
use party::Roster;
use rider::Rider;
use serde::{Deserialize, Serialize};

/// The complete persistent player profile — everything a save file holds
/// about the player (world state lives alongside in the save).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub rider: Rider,
    pub roster: Roster,
    pub inventory: Inventory,
    /// Monotonic id source for creatures and items.
    next_id: u64,
}

impl Profile {
    pub fn new(rider_name: &str, starting_scrip: i64) -> Self {
        Self {
            rider: Rider::new(rider_name),
            roster: Roster::new(),
            inventory: Inventory::new(starting_scrip),
            next_id: 1,
        }
    }

    pub fn allocate_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub fn spawn_creature(
        &mut self,
        data: &GameData,
        species_id: &str,
        origin: CreatureOrigin,
    ) -> u64 {
        let id = self.allocate_id();
        let creature = CreatureInstance::new(id, species_id, origin);
        self.roster.acquire(data, creature);
        id
    }

    pub fn grant_graft(&mut self, def_id: &str, condition: GraftCondition) -> u64 {
        let id = self.allocate_id();
        self.inventory.add(id, def_id, condition)
    }

    /// Equipped item ids across the whole roster (unavailable for staking,
    /// selling, or double-equipping).
    pub fn equipped_item_ids(&self) -> Vec<u64> {
        self.roster
            .creatures
            .iter()
            .flat_map(|c| c.loadout.iter().map(|m| m.item_id))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::GameData;

    #[test]
    fn profile_spawns_creatures_with_unique_ids() {
        let data = GameData::load().unwrap();
        let mut profile = Profile::new("Ash", 200);
        let a = profile.spawn_creature(&data, "volpi", CreatureOrigin::Starter);
        let b = profile.spawn_creature(&data, "bumblit", CreatureOrigin::Wild);
        let item = profile.grant_graft("spark_coil", GraftCondition::Intact);
        assert_ne!(a, b);
        assert_ne!(b, item);
        assert_eq!(profile.roster.party.len(), 2);
    }
}
