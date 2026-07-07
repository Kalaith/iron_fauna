//! Roster and traveling party: the 6-slot size budget (`combat.md` §2.1).

use crate::data::GameData;
use crate::model::creature::CreatureInstance;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Roster {
    /// Every core the player owns, party or banked.
    pub creatures: Vec<CreatureInstance>,
    /// Ids of creatures currently traveling (and fielded in battles).
    pub party: Vec<u64>,
}

impl Roster {
    pub fn new() -> Self {
        Self {
            creatures: Vec::new(),
            party: Vec::new(),
        }
    }

    pub fn creature(&self, id: u64) -> Option<&CreatureInstance> {
        self.creatures.iter().find(|c| c.id == id)
    }

    pub fn creature_mut(&mut self, id: u64) -> Option<&mut CreatureInstance> {
        self.creatures.iter_mut().find(|c| c.id == id)
    }

    pub fn party_members(&self) -> impl Iterator<Item = &CreatureInstance> {
        self.party.iter().filter_map(|id| self.creature(*id))
    }

    /// Creatures banked at settlements (owned but not traveling).
    pub fn stored(&self) -> impl Iterator<Item = &CreatureInstance> {
        self.creatures
            .iter()
            .filter(|c| !self.party.contains(&c.id))
    }

    /// Whether the collection includes at least one core of this species.
    pub fn owns_species(&self, species_id: &str) -> bool {
        self.creatures.iter().any(|c| c.species_id == species_id)
    }

    /// How many cores of this species the collection holds.
    pub fn species_count(&self, species_id: &str) -> usize {
        self.creatures
            .iter()
            .filter(|c| c.species_id == species_id)
            .count()
    }

    pub fn slots_used(&self, data: &GameData) -> u32 {
        self.party_members()
            .map(|c| c.species(data).size.slot_cost())
            .sum()
    }

    pub fn slots_free(&self, data: &GameData) -> u32 {
        data.balance
            .party_slot_budget
            .saturating_sub(self.slots_used(data))
    }

    /// Adds a creature to the roster; joins the party if its size fits the
    /// remaining slot budget, otherwise it goes to storage.
    pub fn acquire(&mut self, data: &GameData, creature: CreatureInstance) -> AcquireResult {
        let cost = creature.species(data).size.slot_cost();
        let id = creature.id;
        let to_party = cost <= self.slots_free(data);
        self.creatures.push(creature);
        if to_party {
            self.party.push(id);
            AcquireResult::JoinedParty
        } else {
            AcquireResult::SentToStorage
        }
    }

    /// Moves a stored creature into the party if the budget allows.
    pub fn add_to_party(&mut self, data: &GameData, id: u64) -> bool {
        if self.party.contains(&id) {
            return false;
        }
        let Some(creature) = self.creature(id) else {
            return false;
        };
        if creature.species(data).size.slot_cost() > self.slots_free(data) {
            return false;
        }
        self.party.push(id);
        true
    }

    pub fn remove_from_party(&mut self, id: u64) -> bool {
        let before = self.party.len();
        self.party.retain(|p| *p != id);
        self.party.len() != before
    }

    /// Shift a party member one slot earlier (`dir == -1`) or later (`dir == 1`).
    /// Party order is the battle line-up and the stance-key order, so the rider
    /// can arrange it. No-op at the ends. Returns whether anything moved.
    pub fn move_in_party(&mut self, id: u64, dir: i32) -> bool {
        let Some(i) = self.party.iter().position(|p| *p == id) else {
            return false;
        };
        let j = i as i32 + dir;
        if j < 0 || j as usize >= self.party.len() {
            return false;
        }
        self.party.swap(i, j as usize);
        true
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcquireResult {
    JoinedParty,
    SentToStorage,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::GameData;
    use crate::model::creature::CreatureOrigin;

    fn creature(id: u64, species: &str) -> CreatureInstance {
        CreatureInstance::new(id, species, CreatureOrigin::Wild)
    }

    #[test]
    fn party_budget_enforced_by_size() {
        let data = GameData::load().unwrap();
        let mut roster = Roster::new();

        // Two Larges (3 + 3) fill the budget of 6.
        assert_eq!(
            roster.acquire(&data, creature(1, "ferrobruin")),
            AcquireResult::JoinedParty
        );
        assert_eq!(
            roster.acquire(&data, creature(2, "ferrobruin")),
            AcquireResult::JoinedParty
        );
        assert_eq!(roster.slots_free(&data), 0);

        // A third catch banks to storage instead.
        assert_eq!(
            roster.acquire(&data, creature(3, "volpi")),
            AcquireResult::SentToStorage
        );
        assert_eq!(roster.stored().count(), 1);

        // Swapping a Large out frees room for the Small.
        assert!(roster.remove_from_party(2));
        assert!(roster.add_to_party(&data, 3));
        assert_eq!(roster.slots_used(&data), 4);
    }
}
