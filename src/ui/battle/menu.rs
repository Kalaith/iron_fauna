//! The command menu: an Atelier/Suikoden-style turn interface layered over the
//! real-time engine. When the ridden creature can act, the field pauses and the
//! player picks a command (attack → weapon → target/part, utility, reinforce,
//! regrow, item, hop, stance). Per-weapon cooldowns are the delay between turns;
//! the clock only advances while the menu is closed.
//!
//! Keyboard handling lives in `menu/input.rs`, rendering in `menu/draw.rs`;
//! this parent owns the menu state, open/close scheduling, and the dynamic
//! option lists both sides consume.

mod draw;
mod input;

use super::BattleScreen;
use crate::combat::{CalledTarget, Side, UnitId, WeaponRef};
use crate::data::GameData;

/// Which panel the command menu is currently showing.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Screen {
    Root,
    /// Pick which weapon (natural or a mount) to fire.
    Weapon,
    /// Pick the enemy and which part of it to strike, for the chosen weapon.
    Target,
    /// Pick a utility graft to trigger.
    Utility,
    /// Pick a potion or ammo to use.
    Item,
    /// Pick which fielded creature to ride.
    Hop,
    /// Flip a party creature's standing orders.
    Stance,
}

pub(super) struct MenuState {
    pub open: bool,
    pub screen: Screen,
    pub cursor: usize,
    /// Weapon chosen on the Weapon screen, awaiting a target on Target.
    pub pending_weapon: Option<WeaponRef>,
    /// Cursor into the target's part list on the Target screen.
    pub part_cursor: usize,
    /// The rider must hop off a cracked mount before anything else — Esc is
    /// disabled while this is set.
    pub forced_hop: bool,
    /// Set the frame a command resolves so a still-ready unit re-prompts
    /// (command chaining) without a manual keypress.
    pub just_acted: bool,
    /// Previous "can act" reading, for rising-edge auto-open in Wait mode.
    pub prev_ready: bool,
}

impl Default for MenuState {
    fn default() -> Self {
        Self {
            open: false,
            screen: Screen::Root,
            cursor: 0,
            pending_weapon: None,
            part_cursor: 0,
            forced_hop: false,
            just_acted: false,
            prev_ready: false,
        }
    }
}

/// A top-level command, filtered to what's currently possible.
#[derive(Clone, Copy)]
enum RootCmd {
    Attack,
    Utility,
    Reinforce,
    Regrow,
    Item,
    Hop,
    Stance,
    Pass,
}

impl RootCmd {
    fn label(self) -> &'static str {
        match self {
            RootCmd::Attack => "Attack",
            RootCmd::Utility => "Utility",
            RootCmd::Reinforce => "Reinforce",
            RootCmd::Regrow => "Regrow limb",
            RootCmd::Item => "Item",
            RootCmd::Hop => "Ride another",
            RootCmd::Stance => "Orders",
            RootCmd::Pass => "Wait",
        }
    }
}

/// A strikeable spot on the target: its body, a bare limb, or a mounted graft.
#[derive(Clone, Copy)]
pub(super) enum Part {
    Center,
    Limb(usize),
    Mount(usize),
}

impl Part {
    fn called(self) -> Option<CalledTarget> {
        match self {
            Part::Center => None,
            Part::Limb(li) => Some(CalledTarget::Limb(li)),
            Part::Mount(mi) => Some(CalledTarget::Mount(mi)),
        }
    }
}

impl BattleScreen {
    // ------------------------------------------------------------------
    // Open / close scheduling
    // ------------------------------------------------------------------

    /// Whether the ridden creature has a cooldown-gated action available right
    /// now. Drives auto-open; always-available actions (hop, stance, item) are
    /// deliberately excluded so readiness actually toggles with the clock.
    fn ridden_ready(&self, data: &GameData) -> bool {
        let Some(id) = self.battle.ridden_unit() else {
            return false;
        };
        if !self.battle.is_commandable(id) {
            return false;
        }
        let u = &self.battle.units[id];
        let weapon_ready = u.natural_cooldown <= 0.0
            || u.weapon_mounts(data)
                .iter()
                .any(|&m| u.mounts[m].cooldown <= 0.0 && u.vigor >= 1.0);
        let reinforce_ready =
            u.reinforce_cooldown <= 0.0 && u.vigor >= data.balance.vigor.reinforce_cost;
        let can_regrow = u.regrow_target.is_none() && u.limbs.iter().any(|l| l.severed);
        let utility_ready = u.mounts.iter().any(|m| {
            m.usable()
                && u.limbs[m.limb_index].intact()
                && m.cooldown <= 0.0
                && data
                    .graftware
                    .get(&m.def_id)
                    .and_then(|d| d.effect)
                    .is_some_and(|e| e.is_triggered())
        });
        weapon_ready || reinforce_ready || can_regrow || utility_ready
    }

    /// Decide whether the menu should open this frame (called before the sim
    /// advances). Handles the forced rider-hop, Wait auto-open, and chaining.
    pub(super) fn schedule_menu(&mut self, data: &GameData) {
        if self.battle.over() {
            self.menu.open = false;
            return;
        }

        // Rider thrown into the open (`combat.md` §7): the only move is to hop.
        if self.battle.rider.exposed() {
            if !self.menu.open {
                self.menu = MenuState {
                    open: true,
                    screen: Screen::Hop,
                    forced_hop: true,
                    ..Default::default()
                };
            }
            self.menu.prev_ready = false;
            return;
        }

        let ready = self.ridden_ready(data);
        if self.pace == crate::state::PaceSetting::Wait
            && !self.menu.open
            && !self.manual_pause
            && ready
            && (self.menu.just_acted || !self.menu.prev_ready)
        {
            self.open_root();
        }
        self.menu.just_acted = false;
        self.menu.prev_ready = ready;
    }

    pub(super) fn open_root(&mut self) {
        self.menu.open = true;
        self.menu.screen = Screen::Root;
        self.menu.cursor = 0;
        self.menu.pending_weapon = None;
        self.menu.forced_hop = false;
    }

    /// A command resolved: close the menu, let the clock breathe, and re-prompt
    /// next frame if the unit can still act.
    fn after_command(&mut self) {
        self.menu.open = false;
        self.menu.pending_weapon = None;
        self.menu.just_acted = true;
    }

    // ------------------------------------------------------------------
    // Dynamic option lists
    // ------------------------------------------------------------------

    fn root_commands(&self, data: &GameData) -> Vec<RootCmd> {
        let mut cmds = vec![RootCmd::Attack];
        let Some(id) = self.battle.ridden_unit() else {
            return vec![RootCmd::Hop];
        };
        let u = &self.battle.units[id];
        let has_utility = u.mounts.iter().any(|m| {
            m.usable()
                && u.limbs[m.limb_index].intact()
                && data
                    .graftware
                    .get(&m.def_id)
                    .and_then(|d| d.effect)
                    .is_some_and(|e| e.is_triggered())
        });
        if has_utility {
            cmds.push(RootCmd::Utility);
        }
        cmds.push(RootCmd::Reinforce);
        if u.limbs.iter().any(|l| l.severed) {
            cmds.push(RootCmd::Regrow);
        }
        if !self.battle.usable_potions(data).is_empty() || !self.battle.usable_ammo(data).is_empty()
        {
            cmds.push(RootCmd::Item);
        }
        if self.battle.alive_on(Side::Player).len() > 1 {
            cmds.push(RootCmd::Hop);
        }
        cmds.push(RootCmd::Stance);
        cmds.push(RootCmd::Pass);
        cmds
    }

    /// Weapons the ridden unit can pick from: natural melee first, then mounts.
    pub(super) fn weapon_list(&self, data: &GameData) -> Vec<WeaponRef> {
        let Some(id) = self.battle.ridden_unit() else {
            return Vec::new();
        };
        let mut list = vec![WeaponRef::Natural];
        for m in self.battle.units[id].weapon_mounts(data) {
            list.push(WeaponRef::Mount(m));
        }
        list
    }

    /// The parts of the current target a shot can be aimed at.
    pub(super) fn part_list(&self) -> Vec<Part> {
        let mut parts = vec![Part::Center];
        let Some(t) = self.battle.units.get(self.target) else {
            return parts;
        };
        for (mi, m) in t.mounts.iter().enumerate() {
            if m.usable() && t.limbs[m.limb_index].intact() {
                parts.push(Part::Mount(mi));
            }
        }
        for li in t.intact_limbs() {
            parts.push(Part::Limb(li));
        }
        parts
    }

    /// Utility grafts the ridden unit can trigger right now.
    fn utility_list(&self, data: &GameData) -> Vec<usize> {
        let Some(id) = self.battle.ridden_unit() else {
            return Vec::new();
        };
        let u = &self.battle.units[id];
        u.mounts
            .iter()
            .enumerate()
            .filter(|(_, m)| {
                m.usable()
                    && u.limbs[m.limb_index].intact()
                    && data
                        .graftware
                        .get(&m.def_id)
                        .and_then(|d| d.effect)
                        .is_some_and(|e| e.is_triggered())
            })
            .map(|(i, _)| i)
            .collect()
    }

    // ------------------------------------------------------------------
    // Small helpers
    // ------------------------------------------------------------------

    pub(super) fn weapon_ready(&self, data: &GameData, weapon: WeaponRef) -> bool {
        let Some(id) = self.battle.ridden_unit() else {
            return false;
        };
        let u = &self.battle.units[id];
        match weapon {
            WeaponRef::Natural => u.natural_cooldown <= 0.0,
            WeaponRef::Mount(m) => {
                u.mounts
                    .get(m)
                    .is_some_and(|mt| mt.usable() && mt.cooldown <= 0.0)
                    && data
                        .graftware
                        .get(&u.mounts[m].def_id)
                        .is_some_and(|d| u.vigor >= d.vigor_cost)
            }
        }
    }

    pub(super) fn ensure_valid_target(&mut self) {
        let alive = self.battle.alive_on(Side::Enemy);
        if !alive.contains(&self.target) {
            if let Some(&first) = alive.first() {
                self.target = first;
            }
        }
    }

    fn hop_targets(&self) -> Vec<UnitId> {
        let current = self.battle.ridden_unit();
        self.battle
            .alive_on(Side::Player)
            .into_iter()
            .filter(|&u| Some(u) != current)
            .collect()
    }

    fn graft_name<'a>(&self, data: &'a GameData, def_id: &'a str) -> &'a str {
        data.graftware
            .get(def_id)
            .map(|d| d.name.as_str())
            .unwrap_or("graft")
    }
}
