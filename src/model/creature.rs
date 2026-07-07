//! Owned creature instances: a caught core plus the loadout built on it.

use crate::data::species::{SpeciesDef, WeightClass};
use crate::data::GameData;
use crate::model::inventory::Inventory;
use serde::{Deserialize, Serialize};

/// Where a core came from — flavors dialogue and the collection screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CreatureOrigin {
    Wild,
    /// Freed from a factory war-body.
    Freed {
        factory_id: String,
    },
    /// Grown in a factory the player has bound.
    Grown {
        factory_id: String,
    },
    Starter,
}

/// One graft item assigned to one mount point on a limb.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MountAssignment {
    pub limb_id: String,
    /// Index into that limb's `mounts` list.
    pub slot: usize,
    pub item_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatureInstance {
    pub id: u64,
    pub species_id: String,
    #[serde(default)]
    pub nickname: Option<String>,
    /// Bond level 0-10; grows with battles fought together.
    pub bond: f32,
    pub loadout: Vec<MountAssignment>,
    pub origin: CreatureOrigin,
}

impl CreatureInstance {
    pub fn new(id: u64, species_id: &str, origin: CreatureOrigin) -> Self {
        Self {
            id,
            species_id: species_id.to_owned(),
            nickname: None,
            bond: 0.0,
            loadout: Vec::new(),
            origin,
        }
    }

    pub fn species<'a>(&self, data: &'a GameData) -> &'a SpeciesDef {
        data.species
            .get(&self.species_id)
            .unwrap_or_else(|| panic!("unknown species id {}", self.species_id))
    }

    pub fn display_name<'a>(&'a self, data: &'a GameData) -> &'a str {
        self.nickname
            .as_deref()
            .unwrap_or_else(|| self.species(data).name.as_str())
    }

    /// Total sustained power draw of the equipped loadout.
    pub fn total_power_draw(&self, data: &GameData, inv: &Inventory) -> f32 {
        self.loadout
            .iter()
            .filter_map(|m| inv.item(m.item_id))
            .filter_map(|item| data.graftware.get(&item.def_id))
            .map(|def| def.power_draw as f32)
            .sum()
    }

    /// Power draw beyond capacity — feeds Strain in battle (`creature.md` §5).
    pub fn overdraw(&self, data: &GameData, inv: &Inventory) -> f32 {
        let capacity = self.species(data).derived(&data.balance).power_capacity;
        (self.total_power_draw(data, inv) - capacity).max(0.0)
    }

    pub fn assignment_at(&self, limb_id: &str, slot: usize) -> Option<&MountAssignment> {
        self.loadout
            .iter()
            .find(|m| m.limb_id == limb_id && m.slot == slot)
    }

    /// What riding this creature unlocks, given its current loadout — the
    /// Boost is species- and kit-dependent (`combat.md` §3.2). One line per
    /// equipped graft that contributes a boost.
    pub fn boost_summary(&self, data: &GameData, inv: &Inventory) -> Vec<String> {
        self.loadout
            .iter()
            .filter_map(|m| inv.item(m.item_id))
            .filter(|i| i.is_usable())
            .filter_map(|i| data.graftware.get(&i.def_id))
            .filter_map(|def| def.boost.describe().map(|d| format!("{}: {}", def.name, d)))
            .collect()
    }

    /// Validates and applies an equip. Overdraw is legal; wrong weight class,
    /// missing mounts, an occupied slot, or an unmet Power floor are not.
    pub fn equip(
        &mut self,
        data: &GameData,
        inv: &Inventory,
        limb_id: &str,
        slot: usize,
        item_id: u64,
    ) -> Result<(), EquipError> {
        let species = self.species(data);
        let limb = species.limb(limb_id).ok_or(EquipError::NoSuchMount)?;
        let mount_class: WeightClass = *limb.mounts.get(slot).ok_or(EquipError::NoSuchMount)?;
        let item = inv.item(item_id).ok_or(EquipError::NoSuchItem)?;
        let def = data
            .graftware
            .get(&item.def_id)
            .ok_or(EquipError::NoSuchItem)?;

        if def.weight > mount_class {
            return Err(EquipError::TooHeavy);
        }
        if species.power < def.min_power {
            return Err(EquipError::PowerFloor(def.min_power));
        }
        if self.assignment_at(limb_id, slot).is_some() {
            return Err(EquipError::SlotOccupied);
        }
        if self.loadout.iter().any(|m| m.item_id == item_id) {
            return Err(EquipError::AlreadyEquipped);
        }

        self.loadout.push(MountAssignment {
            limb_id: limb_id.to_owned(),
            slot,
            item_id,
        });
        Ok(())
    }

    pub fn unequip(&mut self, limb_id: &str, slot: usize) -> Option<u64> {
        let idx = self
            .loadout
            .iter()
            .position(|m| m.limb_id == limb_id && m.slot == slot)?;
        Some(self.loadout.swap_remove(idx).item_id)
    }

    /// Drops assignments whose item no longer exists or is damaged
    /// (destroyed graftware comes off the creature until repaired).
    pub fn prune_loadout(&mut self, inv: &Inventory) {
        self.loadout
            .retain(|m| inv.item(m.item_id).is_some_and(|i| i.is_usable()));
    }

    pub fn add_bond(&mut self, amount: f32) {
        self.bond = (self.bond + amount).min(10.0);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EquipError {
    NoSuchMount,
    NoSuchItem,
    TooHeavy,
    PowerFloor(u32),
    SlotOccupied,
    AlreadyEquipped,
}

impl EquipError {
    pub fn message(self) -> String {
        match self {
            EquipError::NoSuchMount => "No mount point there".to_owned(),
            EquipError::NoSuchItem => "That part is missing".to_owned(),
            EquipError::TooHeavy => "Too heavy for this mount".to_owned(),
            EquipError::PowerFloor(p) => format!("Needs Power {}", p),
            EquipError::SlotOccupied => "Mount already occupied".to_owned(),
            EquipError::AlreadyEquipped => "Part already in use".to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::GameData;
    use crate::model::inventory::{GraftCondition, GraftItem};

    fn setup() -> (GameData, Inventory, CreatureInstance) {
        let data = GameData::load().unwrap();
        let mut inv = Inventory::new(100);
        inv.items.push(GraftItem {
            id: 1,
            def_id: "spark_coil".to_owned(),
            condition: GraftCondition::Intact,
        });
        inv.items.push(GraftItem {
            id: 2,
            def_id: "bolt_cannon".to_owned(),
            condition: GraftCondition::Intact,
        });
        let creature = CreatureInstance::new(10, "volpi", CreatureOrigin::Starter);
        (data, inv, creature)
    }

    #[test]
    fn equip_respects_weight_class_and_power_floor() {
        let (data, inv, mut volpi) = setup();
        // Light coil on a light mount: fine.
        volpi.equip(&data, &inv, "foreleg_l", 0, 1).unwrap();
        // Heavy cannon on a light mount: rejected for weight (and power).
        let err = volpi.equip(&data, &inv, "foreleg_r", 0, 2).unwrap_err();
        assert!(matches!(err, EquipError::TooHeavy));
        // Same slot twice: rejected.
        let err = volpi.equip(&data, &inv, "foreleg_l", 0, 1).unwrap_err();
        assert!(matches!(err, EquipError::SlotOccupied));
    }

    #[test]
    fn boost_summary_reflects_equipped_graftware() {
        let (data, inv, mut volpi) = setup();
        // Nothing equipped: no special boost lines.
        assert!(volpi.boost_summary(&data, &inv).is_empty());
        // Spark coil carries a ChainArc boost — it should surface.
        volpi.equip(&data, &inv, "foreleg_l", 0, 1).unwrap();
        let summary = volpi.boost_summary(&data, &inv);
        assert_eq!(summary.len(), 1);
        assert!(summary[0].contains("Spark Coil"));
        assert!(summary[0].to_lowercase().contains("arc"));
    }

    #[test]
    fn overdraw_is_positive_only_beyond_capacity() {
        let (data, inv, mut volpi) = setup();
        assert_eq!(volpi.overdraw(&data, &inv), 0.0);
        volpi.equip(&data, &inv, "foreleg_l", 0, 1).unwrap();
        // Volpi capacity = 40 * 1.6 = 64; spark coil draws 15 → no overdraw.
        assert_eq!(volpi.overdraw(&data, &inv), 0.0);
        assert!(volpi.total_power_draw(&data, &inv) > 0.0);
    }
}
