//! In-combat consumables: potions (instant, gated by a shared cooldown) and
//! ammunition (loaded into a weapon — a reload that takes a turn). Both are
//! spent from the battle's `bag`, which is reconciled to the inventory after.

use crate::combat::engine::Battle;
use crate::combat::events::BattleEvent;
use crate::combat::unit::LoadedAmmo;
use crate::data::item::ConsumableEffect;
use crate::data::GameData;

/// Shared cooldown after a potion — long enough that a fight can't be spammed
/// to victory, and a Wait-mode decision point in its own right.
const POTION_COOLDOWN: f32 = 2.5;
/// Loading a fresh magazine occupies the weapon — the swap "takes a turn".
const AMMO_RELOAD_TIME: f32 = 2.5;

impl Battle {
    /// Held consumables whose effect matches `pred`, as `(def_id, count)` in the
    /// bag's stable order.
    fn held(&self, data: &GameData, pred: impl Fn(ConsumableEffect) -> bool) -> Vec<(String, u32)> {
        self.bag
            .iter()
            .filter(|(_, &n)| n > 0)
            .filter(|(id, _)| data.items.get(id).is_some_and(|d| pred(d.effect)))
            .map(|(id, &n)| (id.clone(), n))
            .collect()
    }

    pub fn usable_potions(&self, data: &GameData) -> Vec<(String, u32)> {
        self.held(data, |e| e.is_potion())
    }

    pub fn usable_ammo(&self, data: &GameData) -> Vec<(String, u32)> {
        self.held(data, |e| e.is_ammo())
    }

    /// True while a potion may be used (off the shared item cooldown).
    pub fn potion_ready(&self) -> bool {
        self.item_cooldown <= 0.0
    }

    /// Use a potion on the ridden unit. Returns false if not held, the unit
    /// isn't commandable, or the shared cooldown is still running.
    pub fn use_potion(&mut self, data: &GameData, def_id: &str) -> bool {
        let Some(id) = self.ridden_unit() else {
            return false;
        };
        if !self.is_commandable(id) || self.item_cooldown > 0.0 {
            return false;
        }
        if self.bag.get(def_id).copied().unwrap_or(0) == 0 {
            return false;
        }
        let Some(def) = data.items.get(def_id) else {
            return false;
        };
        let cap = self.units[id].core_max * data.balance.vigor.reinforce_shield_cap_frac;
        // Mend and Ward have dedicated float events (they show a number); vigor
        // and strain surface a plain worded callout instead.
        let used = match def.effect {
            ConsumableEffect::MendLimb { amount } => {
                self.heal_unit(id, amount);
                self.events.push(BattleEvent::Healed {
                    source: id,
                    target: id,
                    amount,
                });
                true
            }
            ConsumableEffect::ShieldCore { amount } => {
                let u = &mut self.units[id];
                u.shield = (u.shield + amount).min(cap);
                self.events.push(BattleEvent::Shielded { unit: id, amount });
                true
            }
            ConsumableEffect::RestoreVigor { amount } => {
                let u = &mut self.units[id];
                u.vigor = (u.vigor + amount).min(u.vigor_max);
                self.events.push(BattleEvent::ItemUsed {
                    unit: id,
                    label: format!("+{:.0} vigor", amount),
                });
                true
            }
            ConsumableEffect::VentStrain { amount } => {
                let u = &mut self.units[id];
                u.strain = (u.strain - amount).max(0.0);
                self.events.push(BattleEvent::ItemUsed {
                    unit: id,
                    label: format!("-{:.0} strain", amount),
                });
                true
            }
            _ => false,
        };
        if !used {
            return false;
        }
        self.spend(def_id);
        self.item_cooldown = POTION_COOLDOWN;
        true
    }

    /// Load ammunition into a weapon mount on the ridden unit. The reload
    /// occupies the weapon (`AMMO_RELOAD_TIME`) — the swap costs a turn.
    pub fn load_ammo(&mut self, data: &GameData, mount: usize, def_id: &str) -> bool {
        let Some(id) = self.ridden_unit() else {
            return false;
        };
        if !self.is_commandable(id) {
            return false;
        }
        if self.bag.get(def_id).copied().unwrap_or(0) == 0 {
            return false;
        }
        let Some(ConsumableEffect::Ammo { magazine, .. }) =
            data.items.get(def_id).map(|d| d.effect)
        else {
            return false;
        };
        {
            let u = &self.units[id];
            let Some(m) = u.mounts.get(mount) else {
                return false;
            };
            let is_weapon = data.graftware.get(&m.def_id).is_some_and(|d| d.is_weapon());
            if !m.usable() || !u.limbs[m.limb_index].intact() || !is_weapon {
                return false;
            }
        }
        let m = &mut self.units[id].mounts[mount];
        m.ammo = Some(LoadedAmmo {
            def_id: def_id.to_owned(),
            rounds: magazine,
        });
        m.cooldown = AMMO_RELOAD_TIME;
        let name = data
            .items
            .get(def_id)
            .map(|d| d.name.clone())
            .unwrap_or_default();
        self.events.push(BattleEvent::ItemUsed {
            unit: id,
            label: format!("loaded {}", name),
        });
        self.spend(def_id);
        true
    }

    fn spend(&mut self, def_id: &str) {
        if let Some(n) = self.bag.get_mut(def_id) {
            *n = n.saturating_sub(1);
            if *n == 0 {
                self.bag.remove(def_id);
            }
        }
    }
}
