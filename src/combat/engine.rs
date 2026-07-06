//! The battle simulation: continuous clock, per-unit upkeep, AI dispatch,
//! victory resolution.

use crate::combat::events::BattleEvent;
use crate::combat::unit::{BattleUnit, UnitSpec};
use crate::combat::{
    ai, BattleContext, BattleOutcome, BattleRewards, RiderMods, RiderState, Side, UnitId,
};
use crate::data::graftware::{BoostEffect, GraftEffect};
use crate::data::GameData;
use crate::util::Rng;

pub struct Battle {
    pub context: BattleContext,
    pub units: Vec<BattleUnit>,
    pub rider: RiderState,
    pub time: f32,
    pub events: Vec<BattleEvent>,
    pub outcome: Option<BattleOutcome>,
    /// Enemy graftware knocked loose so far (def ids).
    pub salvage: Vec<String>,
    pub rider_mods: RiderMods,
    pub(crate) rng: Rng,
    pub(crate) ridden_ready_announced: bool,
}

impl Battle {
    pub fn new(
        data: &GameData,
        context: BattleContext,
        player: &[UnitSpec],
        enemy: &[UnitSpec],
        rider_mods: RiderMods,
        seed: u64,
    ) -> Result<Self, String> {
        if player.is_empty() || enemy.is_empty() {
            return Err("battle needs units on both sides".to_owned());
        }
        let mut units = Vec::new();
        for (i, spec) in player.iter().enumerate() {
            units.push(BattleUnit::build(spec, data, -180.0 - i as f32 * 110.0)?);
        }
        for (i, spec) in enemy.iter().enumerate() {
            units.push(BattleUnit::build(spec, data, 180.0 + i as f32 * 110.0)?);
        }
        Ok(Self {
            context,
            units,
            rider: RiderState {
                mounted_on: Some(0),
                hop: None,
            },
            time: 0.0,
            events: Vec::new(),
            outcome: None,
            salvage: Vec::new(),
            rider_mods,
            rng: Rng::new(seed),
            ridden_ready_announced: false,
        })
    }

    pub fn over(&self) -> bool {
        self.outcome.is_some()
    }

    pub fn ridden_unit(&self) -> Option<UnitId> {
        self.rider
            .mounted_on
            .filter(|&id| self.units.get(id).is_some_and(|u| u.alive()))
    }

    /// A unit is player-commandable only while ridden and not berserk.
    pub fn is_commandable(&self, id: UnitId) -> bool {
        self.ridden_unit() == Some(id) && !self.units[id].berserk()
    }

    pub fn alive_on(&self, side: Side) -> Vec<UnitId> {
        self.units
            .iter()
            .enumerate()
            .filter(|(_, u)| u.side == side && u.alive())
            .map(|(i, _)| i)
            .collect()
    }

    pub fn drain_events(&mut self) -> Vec<BattleEvent> {
        std::mem::take(&mut self.events)
    }

    pub fn update(&mut self, data: &GameData, dt: f32) {
        if self.over() || dt <= 0.0 {
            return;
        }
        self.time += dt;

        self.update_hop(dt);
        for id in 0..self.units.len() {
            if self.units[id].alive() {
                self.unit_upkeep(data, id, dt);
            }
        }
        for id in 0..self.units.len() {
            let u = &self.units[id];
            if !u.alive() {
                continue;
            }
            let ridden = self.ridden_unit() == Some(id);
            if !ridden || u.berserk() {
                ai::think(self, data, id);
            }
        }
        self.apply_movement(data, dt);
        self.announce_ridden_ready(data);
        self.check_victory(data);
    }

    fn update_hop(&mut self, dt: f32) {
        if let Some((to, timer)) = self.rider.hop {
            let timer = timer - dt;
            if !self.units.get(to).is_some_and(|u| u.alive()) {
                // Destination cracked mid-transit: rider lands exposed.
                self.rider.hop = None;
                self.events.push(BattleEvent::RiderExposed);
            } else if timer <= 0.0 {
                self.rider.hop = None;
                self.rider.mounted_on = Some(to);
                self.ridden_ready_announced = false;
                self.events.push(BattleEvent::HopLanded { to });
            } else {
                self.rider.hop = Some((to, timer));
            }
        }
    }

    fn unit_upkeep(&mut self, data: &GameData, id: UnitId, dt: f32) {
        let ridden = self.ridden_unit() == Some(id);
        let bal = &data.balance;

        // --- Cooldowns ---
        {
            let u = &mut self.units[id];
            for m in &mut u.mounts {
                m.cooldown = (m.cooldown - dt).max(0.0);
            }
            u.natural_cooldown = (u.natural_cooldown - dt).max(0.0);
            u.reinforce_cooldown = (u.reinforce_cooldown - dt).max(0.0);
            if u.berserk_timer > 0.0 {
                u.berserk_timer -= dt;
                if u.berserk_timer <= 0.0 {
                    u.berserk_timer = 0.0;
                    self.events.push(BattleEvent::BerserkEnded { unit: id });
                }
            }
        }

        // --- Vigor regen (boosted while ridden, `combat.md` §3) ---
        {
            let tap: f32 = self.units[id]
                .working_effects(data)
                .filter_map(|(e, syn)| match e {
                    GraftEffect::VigorTap { per_sec } => Some(per_sec * syn),
                    _ => None,
                })
                .sum();
            let mult = if ridden {
                bal.vigor.ridden_regen_mult
            } else {
                1.0
            };
            let u = &mut self.units[id];
            u.vigor = (u.vigor + (u.vigor_regen_base + tap) * mult * dt).min(u.vigor_max);
        }

        // --- Strain (`game_design.md` §4.3, `combat.md` §6) ---
        {
            let vent: f32 = self.units[id]
                .working_effects(data)
                .filter_map(|(e, syn)| match e {
                    GraftEffect::StrainVent { per_sec } => Some(per_sec * syn),
                    _ => None,
                })
                .sum();
            let overdraw = self.units[id].overdraw(data);
            let mut gain = overdraw * bal.strain.overdraw_gain_per_point;
            if ridden {
                gain += bal.strain.ridden_gain_per_sec * self.rider_mods.strain_gain_mult;
            }
            let u = &mut self.units[id];
            if gain > 0.0 {
                u.strain += gain * dt;
            } else {
                u.strain -= bal.strain.calm_decay_per_sec * dt;
            }
            u.strain = (u.strain - vent * dt).clamp(0.0, u.strain_threshold * 1.25);

            // Once per second at threshold: berserk or graft rejection.
            u.strain_check_accum += dt;
            if u.strain_check_accum >= 1.0 {
                u.strain_check_accum = 0.0;
                if u.strain >= u.strain_threshold {
                    self.strain_episode(data, id);
                }
            }
        }

        self.tick_dots(data, id, dt);
        self.tick_regrow(data, id, dt, ridden);
    }

    /// The creature breaks: berserk episode or graft rejection
    /// (`game_design.md` §4.3).
    fn strain_episode(&mut self, data: &GameData, id: UnitId) {
        let bal = &data.balance;
        let go_berserk = self.rng.chance(bal.strain.berserk_chance);
        let usable_mounts: Vec<usize> = self.units[id]
            .mounts
            .iter()
            .enumerate()
            .filter(|(_, m)| m.usable())
            .map(|(i, _)| i)
            .collect();

        if go_berserk || usable_mounts.is_empty() {
            let u = &mut self.units[id];
            if u.berserk_timer <= 0.0 {
                u.berserk_timer = bal.strain.berserk_duration;
                u.strain *= 0.6;
                self.events.push(BattleEvent::BerserkStarted { unit: id });
            }
        } else {
            let pick = usable_mounts[self.rng.below(usable_mounts.len())];
            let name = self.graft_name(data, id, pick);
            let u = &mut self.units[id];
            u.mounts[pick].destroyed = true;
            u.strain -= u.strain_threshold * bal.strain.rejection_relief_frac;
            u.strain = u.strain.max(0.0);
            self.events.push(BattleEvent::GraftRejected {
                unit: id,
                graft_name: name,
            });
        }
    }

    fn tick_dots(&mut self, data: &GameData, id: UnitId, dt: f32) {
        let mut limb_damage: Vec<(usize, f32)> = Vec::new();
        let mut core_damage = 0.0;
        {
            let u = &mut self.units[id];
            for (limb_index, dot) in &mut u.limb_dots {
                let tick = dot.dps * dt.min(dot.remaining);
                dot.remaining -= dt;
                limb_damage.push((*limb_index, tick));
            }
            u.limb_dots.retain(|(_, d)| d.remaining > 0.0);
            for dot in &mut u.core_dots {
                core_damage += dot.dps * dt.min(dot.remaining);
                dot.remaining -= dt;
            }
            u.core_dots.retain(|d| d.remaining > 0.0);
        }
        for (limb_index, amount) in limb_damage {
            if self.units[id].limbs[limb_index].intact() {
                self.damage_limb_raw(data, id, limb_index, amount);
            }
        }
        if core_damage > 0.0 && self.units[id].core_exposed() {
            self.damage_core_raw(id, core_damage);
        }
    }

    fn tick_regrow(&mut self, data: &GameData, id: UnitId, dt: f32, ridden: bool) {
        let bal = &data.balance;
        let Some(limb_index) = self.units[id].regrow_target else {
            return;
        };
        if !self.units[id].limbs[limb_index].severed {
            self.units[id].regrow_target = None;
            return;
        }
        let growth_mult: f32 = self.units[id]
            .working_effects(data)
            .filter_map(|(e, syn)| match e {
                GraftEffect::GrowthGland { regrow_mult } => Some(regrow_mult * syn),
                _ => None,
            })
            .product::<f32>()
            .max(1.0);
        let boost_mult = if ridden {
            let overgrowth: f32 = self
                .ridden_boosts(data, id)
                .filter_map(|b| match b {
                    BoostEffect::Overgrowth { regrow_mult } => Some(regrow_mult),
                    _ => None,
                })
                .product::<f32>()
                .max(1.0);
            overgrowth * self.rider_mods.ridden_regrow_mult
        } else {
            1.0
        };

        let u = &mut self.units[id];
        let strain_slow = 1.0 - u.strain_frac() * bal.strain.regrow_penalty_at_max;
        let rate = u.regrow_rate * strain_slow.max(0.1) * growth_mult * boost_mult;
        let hp_gain = rate * dt;
        let cost = hp_gain * bal.vigor.regrow_cost_per_hp;
        if u.vigor < cost {
            return; // channel stalls until vigor recovers
        }
        u.vigor -= cost;
        let limb = &mut u.limbs[limb_index];
        limb.regrow_hp += hp_gain;
        if limb.regrow_hp >= limb.max_hp {
            limb.severed = false;
            limb.hp = limb.max_hp;
            limb.regrow_hp = 0.0;
            u.regrow_target = None;
            let name = u.limb_def(data, &u.limbs[limb_index]).name.clone();
            self.events.push(BattleEvent::LimbRegrown {
                unit: id,
                limb_name: name,
            });
        }
    }

    /// Boost effects active on the ridden unit (`combat.md` §3.2).
    pub(crate) fn ridden_boosts<'a>(
        &'a self,
        data: &'a GameData,
        id: UnitId,
    ) -> impl Iterator<Item = BoostEffect> + 'a {
        let u = &self.units[id];
        u.mounts
            .iter()
            .filter(|m| m.usable() && u.limbs[m.limb_index].intact())
            .filter_map(move |m| data.graftware.get(&m.def_id))
            .map(|def| def.boost)
    }

    fn apply_movement(&mut self, data: &GameData, dt: f32) {
        let half = data.balance.battle.arena_half_width;
        for u in &mut self.units {
            if !u.alive() {
                continue;
            }
            u.pos = (u.pos + u.move_intent * u.move_speed * dt).clamp(-half, half);
        }
    }

    fn announce_ridden_ready(&mut self, data: &GameData) {
        let Some(id) = self.ridden_unit() else {
            return;
        };
        let u = &self.units[id];
        let ready = u.natural_cooldown <= 0.0
            || u.weapon_mounts(data)
                .iter()
                .any(|&m| u.mounts[m].cooldown <= 0.0);
        if ready && !self.ridden_ready_announced {
            self.ridden_ready_announced = true;
            self.events
                .push(BattleEvent::RiddenActionReady { unit: id });
        }
    }

    pub(crate) fn graft_name(&self, data: &GameData, unit: UnitId, mount: usize) -> String {
        data.graftware
            .get(&self.units[unit].mounts[mount].def_id)
            .map(|d| d.name.clone())
            .unwrap_or_else(|| "graft".to_owned())
    }

    fn check_victory(&mut self, data: &GameData) {
        if self.over() {
            return;
        }
        if self.alive_on(Side::Enemy).is_empty() {
            let captured_species = if self.context == BattleContext::Duel {
                Vec::new()
            } else {
                self.units
                    .iter()
                    .filter(|u| u.side == Side::Enemy)
                    .map(|u| u.species_id.clone())
                    .collect()
            };
            let scrip: i64 = self
                .units
                .iter()
                .filter(|u| u.side == Side::Enemy)
                .map(|u| 10 + u.species(data).power as i64 / 2)
                .sum();
            self.outcome = Some(BattleOutcome::Victory(BattleRewards {
                captured_species,
                salvage: self.salvage.clone(),
                scrip,
            }));
            self.events.push(BattleEvent::BattleEnded);
        } else if self.alive_on(Side::Player).is_empty() {
            self.outcome = Some(BattleOutcome::Fled);
            self.events.push(BattleEvent::BattleEnded);
        }
    }
}
