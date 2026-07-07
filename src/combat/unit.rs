//! A combatant in a battle: core + war-body limbs + mounted graftware,
//! built from the four-layer anatomy in `game_design.md` §3.

use crate::combat::{Side, Stance};
use crate::data::graftware::{GraftEffect, GraftwareDef};
use crate::data::species::{Element, LimbDef, SpeciesDef, WeightClass};
use crate::data::GameData;

#[derive(Debug, Clone)]
pub struct LimbState {
    /// Index into the species' `limbs` list.
    pub def_index: usize,
    pub hp: f32,
    pub max_hp: f32,
    pub severed: bool,
    /// Regrow progress while severed; limb returns at full HP.
    pub regrow_hp: f32,
}

impl LimbState {
    pub fn intact(&self) -> bool {
        !self.severed
    }
}

/// A magazine of special ammunition loaded into a weapon mount.
#[derive(Debug, Clone)]
pub struct LoadedAmmo {
    /// Consumable def id that was loaded (its stats drive the shot).
    pub def_id: String,
    /// Shots left before the weapon reverts to standard fire.
    pub rounds: u32,
}

#[derive(Debug, Clone)]
pub struct MountState {
    /// Index into the species' `limbs` list for the host limb.
    pub limb_index: usize,
    pub def_id: String,
    /// Inventory item backing this graft (player units only).
    pub item_id: Option<u64>,
    /// Structural HP when targeted directly by called shots.
    pub graft_hp: f32,
    pub graft_hp_max: f32,
    pub destroyed: bool,
    /// Detached with a severed limb — recoverable as salvage, not destroyed.
    pub detached: bool,
    pub cooldown: f32,
    /// Special ammunition loaded into this weapon (None = standard fire).
    pub ammo: Option<LoadedAmmo>,
}

impl MountState {
    pub fn usable(&self) -> bool {
        !self.destroyed && !self.detached
    }
}

/// Damage-over-time effect (venom, burn) chewing on a limb or the core.
#[derive(Debug, Clone)]
pub struct Dot {
    pub dps: f32,
    pub remaining: f32,
}

/// Everything needed to build a battle unit, for either side.
#[derive(Debug, Clone)]
pub struct UnitSpec {
    pub species_id: String,
    pub name: String,
    pub side: Side,
    /// Roster id for player creatures (links battle results back).
    pub creature_id: Option<u64>,
    pub bond: f32,
    pub stance: Stance,
    /// (limb_id, slot, graft def id, backing item id).
    pub grafts: Vec<(String, usize, String, Option<u64>)>,
}

#[derive(Debug, Clone)]
pub struct BattleUnit {
    pub spec_name: String,
    pub side: Side,
    pub creature_id: Option<u64>,
    pub species_id: String,
    pub element: Element,
    pub innate_armor: f32,
    pub limbs: Vec<LimbState>,
    pub mounts: Vec<MountState>,
    pub core_hp: f32,
    pub core_max: f32,
    pub shield: f32,
    pub vigor: f32,
    pub vigor_max: f32,
    pub vigor_regen_base: f32,
    pub strain: f32,
    pub strain_threshold: f32,
    pub power_capacity: f32,
    pub stance: Stance,
    pub pos: f32,
    pub move_speed: f32,
    pub dodge: f32,
    pub accuracy_bonus: f32,
    pub regrow_rate: f32,
    pub downed: bool,
    pub berserk_timer: f32,
    pub natural_cooldown: f32,
    pub natural_damage: f32,
    pub reinforce_cooldown: f32,
    pub core_dots: Vec<Dot>,
    pub limb_dots: Vec<(usize, Dot)>,
    /// Severed limb currently being regrown (channel).
    pub regrow_target: Option<usize>,
    /// -1..1 movement intent; set by AI or by the player when ridden.
    pub move_intent: f32,
    /// Accumulator for once-per-second strain-threshold checks.
    pub strain_check_accum: f32,
}

impl BattleUnit {
    pub fn build(spec: &UnitSpec, data: &GameData, start_pos: f32) -> Result<Self, String> {
        let species = data
            .species
            .get(&spec.species_id)
            .ok_or_else(|| format!("unknown species {}", spec.species_id))?;
        let d = species.derived(&data.balance);
        let bond_vigor = 1.0 + spec.bond * data.balance.curves.bond_vigor_pct;
        let bond_strain = 1.0 + spec.bond * data.balance.curves.bond_strain_pct;

        let limbs: Vec<LimbState> = species
            .limbs
            .iter()
            .enumerate()
            .map(|(i, _)| LimbState {
                def_index: i,
                hp: d.limb_hp,
                max_hp: d.limb_hp,
                severed: false,
                regrow_hp: 0.0,
            })
            .collect();

        let mut mounts = Vec::new();
        for (limb_id, slot, def_id, item_id) in &spec.grafts {
            let limb_index = species
                .limbs
                .iter()
                .position(|l| &l.id == limb_id)
                .ok_or_else(|| format!("{}: no limb {}", spec.species_id, limb_id))?;
            let limb = &species.limbs[limb_index];
            let mount_class: WeightClass = *limb
                .mounts
                .get(*slot)
                .ok_or_else(|| format!("{}: no mount {}/{}", spec.species_id, limb_id, slot))?;
            let def = data
                .graftware
                .get(def_id)
                .ok_or_else(|| format!("unknown graftware {}", def_id))?;
            if def.weight > mount_class {
                return Err(format!("{} too heavy for {}/{}", def_id, limb_id, slot));
            }
            let hp = data
                .balance
                .battle
                .graft_hp
                .get(def.weight)
                .max(def.plating_hp);
            mounts.push(MountState {
                limb_index,
                def_id: def_id.clone(),
                item_id: *item_id,
                graft_hp: hp,
                graft_hp_max: hp,
                destroyed: false,
                detached: false,
                cooldown: 0.0,
                ammo: None,
            });
        }

        Ok(Self {
            spec_name: spec.name.clone(),
            side: spec.side,
            creature_id: spec.creature_id,
            species_id: spec.species_id.clone(),
            element: species.element,
            innate_armor: species.innate_armor as f32,
            limbs,
            mounts,
            core_hp: d.core_hp,
            core_max: d.core_hp,
            shield: 0.0,
            vigor: d.vigor_max * bond_vigor,
            vigor_max: d.vigor_max * bond_vigor,
            vigor_regen_base: d.vigor_regen,
            strain: 0.0,
            strain_threshold: d.strain_threshold * bond_strain,
            power_capacity: d.power_capacity,
            stance: spec.stance,
            pos: start_pos,
            move_speed: d.move_speed,
            dodge: d.dodge,
            accuracy_bonus: d.accuracy_bonus,
            regrow_rate: d.regrow_hp_per_sec,
            downed: false,
            berserk_timer: 0.0,
            natural_cooldown: 0.0,
            natural_damage: species.power as f32 * data.balance.battle.natural_attack_per_power,
            reinforce_cooldown: 0.0,
            core_dots: Vec::new(),
            limb_dots: Vec::new(),
            regrow_target: None,
            move_intent: 0.0,
            strain_check_accum: 0.0,
        })
    }

    pub fn species<'a>(&self, data: &'a GameData) -> &'a SpeciesDef {
        data.species
            .get(&self.species_id)
            .expect("unit species vanished")
    }

    pub fn limb_def<'a>(&self, data: &'a GameData, limb: &LimbState) -> &'a LimbDef {
        &self.species(data).limbs[limb.def_index]
    }

    pub fn alive(&self) -> bool {
        !self.downed
    }

    /// The core is exposed once every limb of the war-body is severed
    /// (`game_design.md` §4.1 step 4).
    pub fn core_exposed(&self) -> bool {
        self.limbs.iter().all(|l| l.severed)
    }

    pub fn berserk(&self) -> bool {
        self.berserk_timer > 0.0
    }

    /// Sum of power draw across working grafts — live load vs capacity.
    pub fn current_draw(&self, data: &GameData) -> f32 {
        self.mounts
            .iter()
            .filter(|m| m.usable())
            .filter_map(|m| data.graftware.get(&m.def_id))
            .map(|g| g.power_draw as f32)
            .sum()
    }

    pub fn overdraw(&self, data: &GameData) -> f32 {
        (self.current_draw(data) - self.power_capacity).max(0.0)
    }

    pub fn strain_frac(&self) -> f32 {
        (self.strain / self.strain_threshold).clamp(0.0, 1.0)
    }

    /// Element synergy multiplier for a graft on this chassis.
    pub fn synergy(&self, data: &GameData, def: &GraftwareDef) -> f32 {
        match def.element {
            Some(e) if e == self.element => data.balance.battle.element_synergy_mult,
            _ => 1.0,
        }
    }

    /// Passive accuracy from sensors, minus strain drift.
    pub fn effective_accuracy_bonus(&self, data: &GameData) -> f32 {
        let sensors: f32 = self
            .working_effects(data)
            .filter_map(|(e, syn)| match e {
                GraftEffect::SensorArray { accuracy_bonus } => Some(accuracy_bonus * syn),
                _ => None,
            })
            .sum();
        let strain_penalty = self.strain_frac() * data.balance.strain.accuracy_penalty_at_max;
        self.accuracy_bonus + sensors - strain_penalty
    }

    /// Iterate working utility effects with their synergy multipliers.
    pub fn working_effects<'a>(
        &'a self,
        data: &'a GameData,
    ) -> impl Iterator<Item = (GraftEffect, f32)> + 'a {
        self.mounts
            .iter()
            .filter(|m| m.usable() && self.limbs[m.limb_index].intact())
            .filter_map(move |m| data.graftware.get(&m.def_id))
            .filter_map(move |def| def.effect.map(|e| (e, self.synergy(data, def))))
    }

    /// Flat armor protecting a limb: innate shell + armor grafts on it.
    pub fn limb_armor(&self, data: &GameData, limb_index: usize) -> f32 {
        let graft_armor: f32 = self
            .mounts
            .iter()
            .filter(|m| m.limb_index == limb_index && m.usable())
            .filter_map(|m| data.graftware.get(&m.def_id))
            .map(|def| def.armor * self.synergy(data, def))
            .sum();
        self.innate_armor + graft_armor
    }

    /// Mount indices that host usable weapons on intact limbs.
    pub fn weapon_mounts(&self, data: &GameData) -> Vec<usize> {
        self.mounts
            .iter()
            .enumerate()
            .filter(|(_, m)| m.usable() && self.limbs[m.limb_index].intact())
            .filter(|(_, m)| data.graftware.get(&m.def_id).is_some_and(|d| d.is_weapon()))
            .map(|(i, _)| i)
            .collect()
    }

    /// Indices of intact (targetable) limbs.
    pub fn intact_limbs(&self) -> Vec<usize> {
        self.limbs
            .iter()
            .enumerate()
            .filter(|(_, l)| l.intact())
            .map(|(i, _)| i)
            .collect()
    }

    pub fn distance_to(&self, other: &BattleUnit) -> f32 {
        (self.pos - other.pos).abs()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::{Side, Stance};
    use crate::data::GameData;

    pub fn spec(species: &str, side: Side, grafts: Vec<(&str, usize, &str)>) -> UnitSpec {
        UnitSpec {
            species_id: species.to_owned(),
            name: species.to_owned(),
            side,
            creature_id: None,
            bond: 0.0,
            stance: Stance::Aggressive,
            grafts: grafts
                .into_iter()
                .map(|(l, s, d)| (l.to_owned(), s, d.to_owned(), None))
                .collect(),
        }
    }

    #[test]
    fn build_wires_limbs_and_mounts() {
        let data = GameData::load().unwrap();
        let unit = BattleUnit::build(
            &spec("volpi", Side::Player, vec![("foreleg_l", 0, "spark_coil")]),
            &data,
            -200.0,
        )
        .unwrap();
        assert_eq!(unit.limbs.len(), 5);
        assert_eq!(unit.mounts.len(), 1);
        assert!(!unit.core_exposed());
        assert_eq!(unit.weapon_mounts(&data).len(), 1);
        assert!(unit.current_draw(&data) > 0.0);
    }

    #[test]
    fn heavy_graft_on_light_mount_is_rejected() {
        let data = GameData::load().unwrap();
        let err = BattleUnit::build(
            &spec("volpi", Side::Player, vec![("foreleg_l", 0, "bolt_cannon")]),
            &data,
            0.0,
        );
        assert!(err.is_err());
    }

    #[test]
    fn core_exposed_when_all_limbs_severed() {
        let data = GameData::load().unwrap();
        let mut unit =
            BattleUnit::build(&spec("volpi", Side::Enemy, vec![]), &data, 200.0).unwrap();
        for limb in &mut unit.limbs {
            limb.severed = true;
        }
        assert!(unit.core_exposed());
        assert!(unit.intact_limbs().is_empty());
    }
}
