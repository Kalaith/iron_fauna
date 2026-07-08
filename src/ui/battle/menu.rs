//! The command menu: an Atelier/Suikoden-style turn interface layered over the
//! real-time engine. When the ridden creature can act, the field pauses and the
//! player picks a command (attack → weapon → target/part, utility, reinforce,
//! regrow, item, hop, stance). Per-weapon cooldowns are the delay between turns;
//! the clock only advances while the menu is closed.

use super::BattleScreen;
use crate::combat::{CalledTarget, PlayerCommand, Side, Stance, UnitId, WeaponRef};
use crate::data::GameData;
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::draw_ui_text_ex;

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
pub(super) enum RootCmd {
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
    pub(super) fn ridden_ready(&self, data: &GameData) -> bool {
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

    pub(super) fn root_commands(&self, data: &GameData) -> Vec<RootCmd> {
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
    pub(super) fn utility_list(&self, data: &GameData) -> Vec<usize> {
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
    // Input
    // ------------------------------------------------------------------

    pub(super) fn handle_menu_input(&mut self, data: &GameData) {
        let up = is_key_pressed(KeyCode::Up);
        let down = is_key_pressed(KeyCode::Down);
        let confirm = is_key_pressed(KeyCode::Enter)
            || is_key_pressed(KeyCode::KpEnter)
            || is_key_pressed(KeyCode::Z);
        let back = is_key_pressed(KeyCode::Escape) || is_key_pressed(KeyCode::Backspace);

        match self.menu.screen {
            Screen::Root => self.input_root(data, up, down, confirm, back),
            Screen::Weapon => self.input_weapon(data, up, down, confirm, back),
            Screen::Target => self.input_target(data, up, down, confirm, back),
            Screen::Utility => self.input_utility(data, up, down, confirm, back),
            Screen::Item => self.input_item(data, up, down, confirm, back),
            Screen::Hop => self.input_hop(data, up, down, confirm, back),
            Screen::Stance => self.input_stance(data, up, down, confirm, back),
        }
    }

    fn step_cursor(cursor: &mut usize, len: usize, up: bool, down: bool) {
        if len == 0 {
            *cursor = 0;
            return;
        }
        if up {
            *cursor = (*cursor + len - 1) % len;
        }
        if down {
            *cursor = (*cursor + 1) % len;
        }
    }

    fn input_root(&mut self, data: &GameData, up: bool, down: bool, confirm: bool, back: bool) {
        let cmds = self.root_commands(data);
        Self::step_cursor(&mut self.menu.cursor, cmds.len(), up, down);
        if back {
            // Esc on the root is "let it ride".
            self.menu.open = false;
            return;
        }
        if !confirm {
            return;
        }
        let Some(&cmd) = cmds.get(self.menu.cursor) else {
            return;
        };
        match cmd {
            RootCmd::Attack => {
                self.menu.screen = Screen::Weapon;
                self.menu.cursor = 0;
            }
            RootCmd::Utility => {
                self.menu.screen = Screen::Utility;
                self.menu.cursor = 0;
            }
            RootCmd::Reinforce => {
                if self.battle.command(data, PlayerCommand::Reinforce) {
                    self.after_command();
                }
            }
            RootCmd::Regrow => {
                if let Some(id) = self.battle.ridden_unit() {
                    let limb = self.battle.units[id].limbs.iter().position(|l| l.severed);
                    if let Some(limb) = limb {
                        if self.battle.command(data, PlayerCommand::Regrow { limb }) {
                            self.after_command();
                        }
                    }
                }
            }
            RootCmd::Item => {
                self.menu.screen = Screen::Item;
                self.menu.cursor = 0;
            }
            RootCmd::Hop => {
                self.menu.screen = Screen::Hop;
                self.menu.cursor = 0;
            }
            RootCmd::Stance => {
                self.menu.screen = Screen::Stance;
                self.menu.cursor = 0;
            }
            RootCmd::Pass => {
                self.menu.open = false;
            }
        }
    }

    fn input_weapon(&mut self, data: &GameData, up: bool, down: bool, confirm: bool, back: bool) {
        let list = self.weapon_list(data);
        Self::step_cursor(&mut self.menu.cursor, list.len(), up, down);
        if back {
            self.menu.screen = Screen::Root;
            return;
        }
        if !confirm {
            return;
        }
        let Some(&weapon) = list.get(self.menu.cursor) else {
            return;
        };
        if !self.weapon_ready(data, weapon) {
            return; // on cooldown / no vigor — pick another
        }
        self.ensure_valid_target();
        self.menu.pending_weapon = Some(weapon);
        self.menu.screen = Screen::Target;
        self.menu.part_cursor = 0;
    }

    fn input_target(&mut self, data: &GameData, up: bool, down: bool, confirm: bool, back: bool) {
        // Left/Right (or Tab) switch which enemy is in the crosshair.
        if is_key_pressed(KeyCode::Left)
            || is_key_pressed(KeyCode::Right)
            || is_key_pressed(KeyCode::Tab)
        {
            let enemies = self.battle.alive_on(Side::Enemy);
            if !enemies.is_empty() {
                let cur = enemies.iter().position(|&e| e == self.target).unwrap_or(0);
                let next = if is_key_pressed(KeyCode::Left) {
                    (cur + enemies.len() - 1) % enemies.len()
                } else {
                    (cur + 1) % enemies.len()
                };
                self.target = enemies[next];
                self.menu.part_cursor = 0;
            }
        }
        let parts = self.part_list();
        Self::step_cursor(&mut self.menu.part_cursor, parts.len(), up, down);
        if back {
            self.menu.screen = Screen::Weapon;
            return;
        }
        if !confirm {
            return;
        }
        let called = parts.get(self.menu.part_cursor).and_then(|p| p.called());
        let Some(weapon) = self.menu.pending_weapon else {
            self.menu.screen = Screen::Root;
            return;
        };
        let cmd = match weapon {
            WeaponRef::Natural => PlayerCommand::NaturalAttack {
                target: self.target,
                called,
            },
            WeaponRef::Mount(mount) => PlayerCommand::Fire {
                mount,
                target: self.target,
                called,
            },
        };
        if self.battle.command(data, cmd) {
            self.after_command();
        }
    }

    fn input_utility(&mut self, data: &GameData, up: bool, down: bool, confirm: bool, back: bool) {
        let list = self.utility_list(data);
        Self::step_cursor(&mut self.menu.cursor, list.len(), up, down);
        if back {
            self.menu.screen = Screen::Root;
            return;
        }
        if !confirm {
            return;
        }
        if let Some(&mount) = list.get(self.menu.cursor) {
            if self
                .battle
                .command(data, PlayerCommand::TriggerUtility { mount, ally: None })
            {
                self.after_command();
            }
        }
    }

    fn input_item(&mut self, data: &GameData, up: bool, down: bool, confirm: bool, back: bool) {
        let potions = self.battle.usable_potions(data);
        let ammo = self.battle.usable_ammo(data);
        let len = potions.len() + ammo.len();
        Self::step_cursor(&mut self.menu.cursor, len.max(1), up, down);
        if back {
            self.menu.screen = Screen::Root;
            return;
        }
        if !confirm {
            return;
        }
        let i = self.menu.cursor;
        let used = if i < potions.len() {
            self.battle.use_potion(data, &potions[i].0.clone())
        } else if let Some((def_id, _)) = ammo.get(i - potions.len()).cloned() {
            self.load_first_weapon(data, &def_id)
        } else {
            false
        };
        if used {
            self.after_command();
        }
    }

    fn input_hop(&mut self, data: &GameData, up: bool, down: bool, confirm: bool, back: bool) {
        let list = self.hop_targets();
        Self::step_cursor(&mut self.menu.cursor, list.len(), up, down);
        if back && !self.menu.forced_hop {
            self.menu.screen = Screen::Root;
            return;
        }
        if !confirm {
            return;
        }
        if let Some(&to) = list.get(self.menu.cursor) {
            // BeginHop routes through the engine even while the rider is exposed.
            if self.battle.command(data, PlayerCommand::BeginHop { to }) {
                self.after_command();
                self.menu.forced_hop = false;
            }
        }
    }

    fn input_stance(&mut self, data: &GameData, up: bool, down: bool, confirm: bool, back: bool) {
        let allies = self.battle.alive_on(Side::Player);
        Self::step_cursor(&mut self.menu.cursor, allies.len(), up, down);
        if back {
            self.menu.screen = Screen::Root;
            return;
        }
        if !confirm {
            return;
        }
        if let Some(&unit) = allies.get(self.menu.cursor) {
            let stance = match self.battle.units[unit].stance {
                Stance::Aggressive => Stance::Defensive,
                Stance::Defensive => Stance::Aggressive,
            };
            // Orders are free to set — stay on the panel, don't consume the turn.
            let _ = self
                .battle
                .command(data, PlayerCommand::SetStance { unit, stance });
        }
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

    pub(super) fn hop_targets(&self) -> Vec<UnitId> {
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

    // ------------------------------------------------------------------
    // Drawing
    // ------------------------------------------------------------------

    pub(super) fn draw_menu(&self, data: &GameData) {
        let (title, rows, cursor, hint) = self.menu_rows(data);
        let panel_h =
            66.0 + rows.len().max(1) as f32 * 26.0 + if hint.is_some() { 22.0 } else { 0.0 };
        let rect = Rect::new(
            24.0,
            crate::ui::LOGICAL_HEIGHT - panel_h - 24.0,
            430.0,
            panel_h,
        );
        draw_surface(
            rect,
            &SurfaceStyle::new(Color::new(0.05, 0.07, 0.09, 0.96))
                .with_border(1.5, Color::new(0.4, 0.6, 0.75, 0.85)),
        );
        draw_ui_text_ex(
            &title,
            rect.x + 16.0,
            rect.y + 28.0,
            TextStyle::new(18.0, Color::new(0.85, 0.92, 0.98, 1.0)).params(),
        );
        let mut y = rect.y + 56.0;
        if rows.is_empty() {
            draw_ui_text_ex(
                "— nothing available —",
                rect.x + 20.0,
                y,
                TextStyle::new(15.0, dark::TEXT_DIM).params(),
            );
        }
        for (i, (label, enabled)) in rows.iter().enumerate() {
            let selected = i == cursor;
            if selected {
                draw_rectangle(
                    rect.x + 8.0,
                    y - 16.0,
                    rect.w - 16.0,
                    24.0,
                    Color::new(0.2, 0.32, 0.42, 0.9),
                );
            }
            let color = if !enabled {
                dark::TEXT_DIM
            } else if selected {
                Color::new(0.98, 0.98, 0.9, 1.0)
            } else {
                dark::TEXT
            };
            draw_ui_text_ex(
                &format!("{} {}", if selected { ">" } else { " " }, label),
                rect.x + 16.0,
                y,
                TextStyle::new(16.0, color).params(),
            );
            y += 26.0;
        }
        if let Some(hint) = hint {
            draw_ui_text_ex(
                &hint,
                rect.x + 16.0,
                y + 4.0,
                TextStyle::new(13.0, dark::TEXT_DIM).params(),
            );
        }
    }

    /// (title, [(row label, enabled)], cursor index, optional hint) for the
    /// current screen.
    fn menu_rows(&self, data: &GameData) -> (String, Vec<(String, bool)>, usize, Option<String>) {
        match self.menu.screen {
            Screen::Root => {
                let name = self
                    .battle
                    .ridden_unit()
                    .map(|id| self.battle.units[id].spec_name.as_str())
                    .unwrap_or("—");
                let rows = self
                    .root_commands(data)
                    .into_iter()
                    .map(|c| (c.label().to_owned(), true))
                    .collect();
                (
                    format!("{} — orders", name),
                    rows,
                    self.menu.cursor,
                    Some("[Up/Dn] choose · [Enter] confirm · [Esc] let it ride".to_owned()),
                )
            }
            Screen::Weapon => {
                let rows = self
                    .weapon_list(data)
                    .into_iter()
                    .map(|w| self.weapon_row(data, w))
                    .collect();
                (
                    "Attack with…".to_owned(),
                    rows,
                    self.menu.cursor,
                    Some("[Up/Dn] choose · [Enter] aim · [Esc] back".to_owned()),
                )
            }
            Screen::Target => {
                let tname = self
                    .battle
                    .units
                    .get(self.target)
                    .map(|u| u.spec_name.clone())
                    .unwrap_or_default();
                let rows = self
                    .part_list()
                    .into_iter()
                    .map(|p| (self.part_label(data, p), true))
                    .collect();
                (
                    format!("Strike {} —", tname),
                    rows,
                    self.menu.part_cursor,
                    Some(
                        "[<-/->] switch foe · [Up/Dn] pick spot · [Enter] fire · [Esc] back"
                            .to_owned(),
                    ),
                )
            }
            Screen::Utility => {
                let rows = self
                    .utility_list(data)
                    .into_iter()
                    .map(|m| {
                        let u = &self.battle.units[self.battle.ridden_unit().unwrap_or(0)];
                        let ready = u.mounts[m].cooldown <= 0.0;
                        let name = self.graft_name(data, &u.mounts[m].def_id).to_owned();
                        let status = if ready {
                            "ready".to_owned()
                        } else {
                            format!("{:.1}s", u.mounts[m].cooldown)
                        };
                        (format!("{} — {}", name, status), ready)
                    })
                    .collect();
                (
                    "Utility".to_owned(),
                    rows,
                    self.menu.cursor,
                    Some("Enter use · Esc back".to_owned()),
                )
            }
            Screen::Item => {
                let mut rows: Vec<(String, bool)> = Vec::new();
                let ready = self.battle.potion_ready();
                for (def_id, count) in self.battle.usable_potions(data) {
                    let name = data
                        .items
                        .get(&def_id)
                        .map(|d| d.name.as_str())
                        .unwrap_or("?");
                    rows.push((format!("{} ×{}  (potion)", name, count), ready));
                }
                for (def_id, count) in self.battle.usable_ammo(data) {
                    let name = data
                        .items
                        .get(&def_id)
                        .map(|d| d.name.as_str())
                        .unwrap_or("?");
                    rows.push((format!("{} ×{}  (load weapon)", name, count), true));
                }
                (
                    "Items".to_owned(),
                    rows,
                    self.menu.cursor,
                    Some("Enter use · Esc back".to_owned()),
                )
            }
            Screen::Hop => {
                let rows = self
                    .hop_targets()
                    .into_iter()
                    .map(|id| {
                        let u = &self.battle.units[id];
                        (
                            format!(
                                "{}  (core {:.0}%)",
                                u.spec_name,
                                u.core_hp / u.core_max * 100.0
                            ),
                            true,
                        )
                    })
                    .collect();
                let hint = if self.menu.forced_hop {
                    "Rider exposed — choose a mount · Enter ride"
                } else {
                    "Enter ride · Esc back"
                };
                (
                    "Ride which creature?".to_owned(),
                    rows,
                    self.menu.cursor,
                    Some(hint.to_owned()),
                )
            }
            Screen::Stance => {
                let rows = self
                    .battle
                    .alive_on(Side::Player)
                    .into_iter()
                    .map(|id| {
                        let u = &self.battle.units[id];
                        (
                            format!("{} — {}", u.spec_name, u.stance.display_name()),
                            true,
                        )
                    })
                    .collect();
                (
                    "Standing orders".to_owned(),
                    rows,
                    self.menu.cursor,
                    Some("Enter toggle · Esc back".to_owned()),
                )
            }
        }
    }

    fn weapon_row(&self, data: &GameData, weapon: WeaponRef) -> (String, bool) {
        let ready = self.weapon_ready(data, weapon);
        let Some(id) = self.battle.ridden_unit() else {
            return ("—".to_owned(), false);
        };
        let u = &self.battle.units[id];
        match weapon {
            WeaponRef::Natural => {
                let status = if u.natural_cooldown <= 0.0 {
                    "ready".to_owned()
                } else {
                    format!("{:.1}s", u.natural_cooldown)
                };
                (format!("Natural strike — {}", status), ready)
            }
            WeaponRef::Mount(m) => {
                let name = self.graft_name(data, &u.mounts[m].def_id).to_owned();
                let dmg = data
                    .graftware
                    .get(&u.mounts[m].def_id)
                    .map(|d| d.damage)
                    .unwrap_or(0.0);
                let status = if u.mounts[m].cooldown <= 0.0 {
                    "ready".to_owned()
                } else {
                    format!("{:.1}s", u.mounts[m].cooldown)
                };
                (format!("{} ({:.0} dmg) — {}", name, dmg, status), ready)
            }
        }
    }

    fn part_label(&self, data: &GameData, part: Part) -> String {
        let Some(t) = self.battle.units.get(self.target) else {
            return "—".to_owned();
        };
        match part {
            Part::Center => "center mass".to_owned(),
            Part::Limb(li) => t
                .limbs
                .get(li)
                .map(|l| t.limb_def(data, l).name.clone())
                .unwrap_or_else(|| "limb".to_owned()),
            Part::Mount(mi) => match t.mounts.get(mi) {
                Some(m) => format!(
                    "{} ({:.0}%)",
                    self.graft_name(data, &m.def_id),
                    (m.graft_hp / m.graft_hp_max * 100.0).clamp(0.0, 100.0)
                ),
                None => "graft".to_owned(),
            },
        }
    }
}
