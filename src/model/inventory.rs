//! Player inventory: graftware items, salvage, and currency.

use crate::data::GameData;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GraftCondition {
    Intact,
    /// Destroyed in battle — unusable until repaired (`game_design.md` §4.4).
    Damaged,
}

/// One physical graftware part. All owned parts live in the inventory;
/// creature loadouts reference them by id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraftItem {
    pub id: u64,
    pub def_id: String,
    pub condition: GraftCondition,
}

impl GraftItem {
    pub fn is_usable(&self) -> bool {
        self.condition == GraftCondition::Intact
    }
}

/// Stackable consumables (potions, ammo, field kits) as `def_id → count`. A
/// `BTreeMap` keeps a stable, sorted order for the UI and serialises cleanly.
pub type ConsumableBag = std::collections::BTreeMap<String, u32>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Inventory {
    pub items: Vec<GraftItem>,
    /// Settlement scrip — the survivors' currency.
    pub scrip: i64,
    /// Spendable consumables by def id.
    #[serde(default)]
    pub consumables: ConsumableBag,
}

impl Inventory {
    pub fn new(scrip: i64) -> Self {
        Self {
            items: Vec::new(),
            scrip,
            consumables: ConsumableBag::new(),
        }
    }

    pub fn add_consumable(&mut self, def_id: &str, count: u32) {
        *self.consumables.entry(def_id.to_owned()).or_insert(0) += count;
    }

    pub fn consumable_count(&self, def_id: &str) -> u32 {
        self.consumables.get(def_id).copied().unwrap_or(0)
    }

    /// Spend one of a consumable. Returns false if none are held.
    pub fn take_consumable(&mut self, def_id: &str) -> bool {
        match self.consumables.get_mut(def_id) {
            Some(n) if *n > 0 => {
                *n -= 1;
                if *n == 0 {
                    self.consumables.remove(def_id);
                }
                true
            }
            _ => false,
        }
    }

    pub fn item(&self, id: u64) -> Option<&GraftItem> {
        self.items.iter().find(|i| i.id == id)
    }

    pub fn item_mut(&mut self, id: u64) -> Option<&mut GraftItem> {
        self.items.iter_mut().find(|i| i.id == id)
    }

    pub fn add(&mut self, id: u64, def_id: &str, condition: GraftCondition) -> u64 {
        self.items.push(GraftItem {
            id,
            def_id: def_id.to_owned(),
            condition,
        });
        id
    }

    pub fn mark_damaged(&mut self, id: u64) {
        if let Some(item) = self.item_mut(id) {
            item.condition = GraftCondition::Damaged;
        }
    }

    /// Repairs a damaged part for scrip. Returns false if unaffordable,
    /// missing, or not damaged.
    pub fn repair(&mut self, data: &GameData, id: u64) -> bool {
        let Some(item) = self.item(id) else {
            return false;
        };
        if item.condition != GraftCondition::Damaged {
            return false;
        }
        let Some(def) = data.graftware.get(&item.def_id) else {
            return false;
        };
        let cost = def.repair_cost();
        if self.scrip < cost {
            return false;
        }
        self.scrip -= cost;
        if let Some(item) = self.item_mut(id) {
            item.condition = GraftCondition::Intact;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::GameData;

    #[test]
    fn repair_costs_scrip_and_restores_condition() {
        let data = GameData::load().unwrap();
        let mut inv = Inventory::new(1000);
        inv.add(1, "spark_coil", GraftCondition::Damaged);

        assert!(inv.repair(&data, 1));
        assert_eq!(inv.item(1).unwrap().condition, GraftCondition::Intact);
        assert!(inv.scrip < 1000);

        // Repairing an intact item is a no-op failure.
        assert!(!inv.repair(&data, 1));
    }

    #[test]
    fn repair_fails_when_broke() {
        let data = GameData::load().unwrap();
        let mut inv = Inventory::new(0);
        inv.add(1, "bolt_cannon", GraftCondition::Damaged);
        assert!(!inv.repair(&data, 1));
        assert_eq!(inv.item(1).unwrap().condition, GraftCondition::Damaged);
    }
}
