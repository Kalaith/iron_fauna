//! The battle screen: a fixed-position side-view field with an Atelier-style
//! command menu. The player directly commands one ridden creature a turn at a
//! time (menu in `menu.rs`); the rest of the party fights on standing orders.
//! The real-time clock only advances while the menu is closed.

mod events_view;
mod hud;
mod menu;

use crate::audio::Audio;
use crate::combat::engine::Battle;
use crate::combat::events::BattleEvent;
use crate::combat::{Side, UnitId, WeaponRef};
use crate::data::species::SizeClass;
use crate::data::GameData;
use crate::state::PaceSetting;
use crate::ui::{creature_art, LOGICAL_HEIGHT, LOGICAL_WIDTH};
use events_view::{describe_event, sfx_for_event};
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::draw_ui_text_ex;
use menu::{MenuState, Screen};

const GROUND_Y: f32 = 470.0;
const LOG_LINES: usize = 6;

pub enum BattleScreenResult {
    Continue,
    /// Battle resolved and acknowledged — return to the previous state.
    Finished,
}

pub struct BattleScreen {
    pub battle: Battle,
    pub pace: PaceSetting,
    manual_pause: bool,
    /// The enemy currently in the crosshair.
    target: UnitId,
    menu: MenuState,
    log: Vec<String>,
    /// Rising, fading combat text spawned from battle events.
    floats: Vec<FloatText>,
    outcome_shown: bool,
}

/// One piece of floating combat text (damage, "MISS", "SEVERED!", …).
struct FloatText {
    x: f32,
    y: f32,
    text: String,
    color: Color,
    age: f32,
    ttl: f32,
}

impl BattleScreen {
    pub fn new(battle: Battle, pace: PaceSetting) -> Self {
        let target = battle.alive_on(Side::Enemy).first().copied().unwrap_or(0);
        Self {
            battle,
            pace,
            manual_pause: false,
            target,
            menu: MenuState::default(),
            log: vec!["The shells close in.".to_owned()],
            floats: Vec::new(),
            outcome_shown: false,
        }
    }

    /// Test/capture hook: open targeting with a weapon chosen, for a screenshot.
    pub fn force_aim_capture(&mut self, data: &GameData) {
        self.ensure_valid_target();
        let weapon = self
            .weapon_list(data)
            .into_iter()
            .find(|&w| self.weapon_ready(data, w))
            .unwrap_or(WeaponRef::Natural);
        self.menu.open = true;
        self.menu.screen = Screen::Target;
        self.menu.pending_weapon = Some(weapon);
        // Highlight a called part (not center mass) if the target has one.
        self.menu.part_cursor = if self.part_list().len() > 1 { 1 } else { 0 };
        self.manual_pause = true;
    }

    /// Test/capture hook: open the item panel for a screenshot.
    pub fn force_item_capture(&mut self) {
        self.menu.open = true;
        self.menu.screen = Screen::Item;
        self.menu.cursor = 0;
        self.manual_pause = true;
    }

    fn paused(&self) -> bool {
        self.manual_pause || self.menu.open
    }

    pub fn update(&mut self, data: &GameData, audio: &Audio, dt: f32) -> BattleScreenResult {
        if self.battle.over() {
            return self.update_outcome();
        }

        if is_key_pressed(KeyCode::P) {
            self.manual_pause = !self.manual_pause;
        }

        // The menu can only take input if it was already open at frame start —
        // that keeps the key that *opens* it from also confirming a selection.
        let was_open = self.menu.open;

        // Manual open (the only way in Active mode; a convenience after Wait's
        // "let it ride"): Enter/Space summons the command menu.
        if !was_open
            && !self.manual_pause
            && self.commandable()
            && (is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::Space))
        {
            self.open_root();
        }

        self.ensure_valid_target();
        self.schedule_menu(data);

        if self.menu.open && was_open {
            self.handle_menu_input(data);
        }

        if !self.paused() {
            self.battle.update(data, dt);
        }

        for event in self.battle.drain_events() {
            if let Some(sfx) = sfx_for_event(&event) {
                audio.play(sfx);
            }
            self.spawn_float(data, &event);
            if let Some(line) = describe_event(&self.battle, data, &event) {
                self.log.push(line);
            }
        }
        if self.log.len() > 40 {
            let excess = self.log.len() - 40;
            self.log.drain(..excess);
        }

        // Age and retire floating text (only advances with the clock).
        if !self.paused() {
            for f in &mut self.floats {
                f.age += dt;
                f.y -= dt * 34.0;
            }
            self.floats.retain(|f| f.age < f.ttl);
        }
        BattleScreenResult::Continue
    }

    fn commandable(&self) -> bool {
        self.battle
            .ridden_unit()
            .is_some_and(|id| self.battle.is_commandable(id))
    }

    fn update_outcome(&mut self) -> BattleScreenResult {
        self.outcome_shown = true;
        if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::Space) {
            return BattleScreenResult::Finished;
        }
        BattleScreenResult::Continue
    }

    /// Turns a battle event into a floating callout over the unit involved.
    fn spawn_float(&mut self, data: &GameData, event: &BattleEvent) {
        let at = |screen: &Self, id: UnitId| -> (f32, f32) {
            let (x, y) = screen.unit_slot(data, id);
            (x, y - screen.size_radius(data, id) - 30.0)
        };
        let (x, y, text, color) = match event {
            BattleEvent::Hit {
                target,
                amount,
                to_core,
                ..
            } => {
                let (x, y) = at(self, *target);
                let col = if *to_core {
                    Color::new(1.0, 0.55, 0.45, 1.0)
                } else {
                    Color::new(0.95, 0.9, 0.8, 1.0)
                };
                (x, y, format!("{:.0}", amount), col)
            }
            BattleEvent::Miss { target, .. } => {
                let (x, y) = at(self, *target);
                (x, y, "miss".to_owned(), Color::new(0.6, 0.65, 0.7, 1.0))
            }
            BattleEvent::LimbSevered { unit, .. } => {
                let (x, y) = at(self, *unit);
                (x, y, "SEVERED".to_owned(), Color::new(0.95, 0.4, 0.3, 1.0))
            }
            BattleEvent::GraftDestroyed { unit, .. } => {
                let (x, y) = at(self, *unit);
                (
                    x,
                    y,
                    "graft down".to_owned(),
                    Color::new(0.9, 0.5, 0.35, 1.0),
                )
            }
            BattleEvent::GraftRejected { unit, .. } => {
                let (x, y) = at(self, *unit);
                (
                    x,
                    y,
                    "REJECTED".to_owned(),
                    Color::new(0.95, 0.3, 0.25, 1.0),
                )
            }
            BattleEvent::BerserkStarted { unit } => {
                let (x, y) = at(self, *unit);
                (x, y, "BERSERK".to_owned(), Color::new(0.98, 0.35, 0.2, 1.0))
            }
            BattleEvent::Healed { target, amount, .. } => {
                let (x, y) = at(self, *target);
                (
                    x,
                    y,
                    format!("+{:.0}", amount),
                    Color::new(0.5, 0.85, 0.5, 1.0),
                )
            }
            BattleEvent::Shielded { unit, .. } => {
                let (x, y) = at(self, *unit);
                (x, y, "shield".to_owned(), Color::new(0.5, 0.75, 0.95, 1.0))
            }
            BattleEvent::CoreExposed { unit } => {
                let (x, y) = at(self, *unit);
                (x, y, "EXPOSED".to_owned(), Color::new(1.0, 0.9, 0.5, 1.0))
            }
            BattleEvent::CoreCracked { unit } => {
                let (x, y) = at(self, *unit);
                (x, y, "CRACKED".to_owned(), Color::new(1.0, 0.95, 0.7, 1.0))
            }
            BattleEvent::ItemUsed { unit, label } => {
                let (x, y) = at(self, *unit);
                (x, y, label.clone(), Color::new(0.65, 0.9, 0.7, 1.0))
            }
            _ => return,
        };
        // Cap concurrent floats so a flurry can't clutter the field.
        if self.floats.len() > 24 {
            self.floats.remove(0);
        }
        self.floats.push(FloatText {
            x,
            y,
            text,
            color,
            age: 0.0,
            ttl: 0.9,
        });
    }

    // ------------------------------------------------------------------
    // Drawing
    // ------------------------------------------------------------------

    pub fn draw(&self, data: &GameData) {
        self.draw_backdrop();
        for (id, _) in self.battle.units.iter().enumerate() {
            self.draw_unit(data, id);
        }
        self.draw_floats();
        self.draw_hud(data);
        if self.menu.open {
            self.draw_menu(data);
        }
        self.draw_log();
        if self.battle.over() {
            self.draw_outcome(data);
        } else if self.manual_pause {
            self.draw_pause_banner();
        }
    }

    fn draw_floats(&self) {
        for f in &self.floats {
            let t = (f.age / f.ttl).clamp(0.0, 1.0);
            let alpha = (1.0 - t).clamp(0.0, 1.0);
            let size = if f.text.chars().all(|c| c.is_ascii_digit() || c == '+') {
                22.0
            } else {
                18.0
            };
            let col = Color::new(f.color.r, f.color.g, f.color.b, alpha);
            // Cheap shadow for legibility over any backdrop.
            draw_ui_text_ex(
                &f.text,
                f.x - f.text.len() as f32 * 3.0 + 1.0,
                f.y + 1.0,
                TextStyle::new(size, Color::new(0.0, 0.0, 0.0, alpha * 0.7)).params(),
            );
            draw_ui_text_ex(
                &f.text,
                f.x - f.text.len() as f32 * 3.0,
                f.y,
                TextStyle::new(size, col).params(),
            );
        }
    }

    /// The fixed screen slot for a unit: player party on the left, enemies on
    /// the right, each side fanned vertically with a small back-rank stagger.
    fn unit_slot(&self, _data: &GameData, id: UnitId) -> (f32, f32) {
        let side = self.battle.units[id].side;
        let peers: Vec<UnitId> = self
            .battle
            .units
            .iter()
            .enumerate()
            .filter(|(_, u)| u.side == side)
            .map(|(i, _)| i)
            .collect();
        let idx = peers.iter().position(|&p| p == id).unwrap_or(0);
        let n = peers.len().max(1);
        let (base_x, stagger) = match side {
            Side::Player => (300.0, 1.0),
            Side::Enemy => (980.0, -1.0),
        };
        let x = base_x + (idx % 2) as f32 * 56.0 * stagger;
        let (top, bottom) = (180.0, 400.0);
        let y = if n <= 1 {
            (top + bottom) * 0.5
        } else {
            top + (bottom - top) * (idx as f32 / (n - 1) as f32)
        };
        (x, y)
    }

    fn draw_backdrop(&self) {
        draw_rectangle(
            0.0,
            0.0,
            LOGICAL_WIDTH,
            GROUND_Y,
            Color::new(0.07, 0.075, 0.10, 1.0),
        );
        draw_rectangle(
            0.0,
            GROUND_Y,
            LOGICAL_WIDTH,
            LOGICAL_HEIGHT - GROUND_Y,
            Color::new(0.10, 0.10, 0.09, 1.0),
        );
        draw_line(
            0.0,
            GROUND_Y,
            LOGICAL_WIDTH,
            GROUND_Y,
            2.0,
            Color::new(0.3, 0.3, 0.28, 1.0),
        );
    }

    fn size_radius(&self, data: &GameData, id: UnitId) -> f32 {
        match self.battle.units[id].species(data).size {
            SizeClass::Small => 20.0,
            SizeClass::Medium => 30.0,
            SizeClass::Large => 42.0,
            SizeClass::Huge => 54.0,
        }
    }

    fn draw_unit(&self, data: &GameData, id: UnitId) {
        let u = &self.battle.units[id];
        let (x, y) = self.unit_slot(data, id);
        let r = self.size_radius(data, id);
        let species = u.species(data);

        // Grafts still riding on intact limbs, drawn by the shared procedural
        // renderer — one creature look across bench, bestiary, and battle.
        let grafts: Vec<creature_art::GraftVisual> = u
            .mounts
            .iter()
            .filter_map(|m| {
                let limb = u.limbs.get(m.limb_index)?;
                if !limb.intact() {
                    return None; // it went down with the limb
                }
                let region = u.limb_def(data, limb).region;
                let def = data.graftware.get(&m.def_id)?;
                Some(creature_art::GraftVisual {
                    region,
                    kind: def.kind,
                    element: def.element,
                    broken: !m.usable(),
                })
            })
            .collect();

        // Core-exposed warm glow behind the head — the vulnerable moment.
        if u.core_exposed() && !u.downed {
            use crate::data::species::LimbRegion;
            let head = creature_art::region_point(x, y, r, LimbRegion::Head);
            let pulse = 0.22 + 0.12 * (self.battle.time * 5.0).sin().abs();
            draw_circle(head.x, head.y, r * 0.6, Color::new(1.0, 0.9, 0.55, pulse));
        }

        if u.downed {
            creature_art::draw_downed(x, y, r, species, &grafts);
        } else {
            creature_art::draw_war_body(x, y, r, species, &grafts);
        }

        // Severed limbs: a raw red stump at the region's anchor, regrowth
        // filling it back green (`combat.md` §3.1, §4.2).
        for (i, limb) in u.limbs.iter().enumerate() {
            if limb.intact() {
                continue;
            }
            let region = u.limb_def(data, limb).region;
            let p = creature_art::region_point(x, y, r, region);
            draw_circle(p.x, p.y, 6.0, Color::new(0.4, 0.12, 0.12, 0.95));
            let frac = (limb.regrow_hp / limb.max_hp).clamp(0.0, 1.0);
            if u.regrow_target == Some(i) || frac > 0.0 {
                draw_circle(
                    p.x,
                    p.y,
                    6.0 * frac.max(0.15),
                    Color::new(0.4, 0.6, 0.35, 0.95),
                );
            }
        }

        // Shield ring.
        if u.shield > 0.0 {
            draw_circle_lines(x, y, r + 8.0, 2.0, Color::new(0.5, 0.75, 0.95, 0.8));
        }
        // Berserk flare.
        if u.berserk() {
            draw_circle_lines(x, y, r + 12.0, 2.5, Color::new(0.95, 0.3, 0.2, 0.9));
        }

        // Name + bars.
        let name_style = if u.side == Side::Player {
            TextStyle::new(14.0, Color::new(0.7, 0.85, 0.95, 1.0))
        } else {
            TextStyle::new(14.0, Color::new(0.95, 0.7, 0.65, 1.0))
        };
        draw_ui_text_ex(&u.spec_name, x - r, y - r - 26.0, name_style.params());
        let bar_w = (r * 2.0).max(48.0);
        draw_bar(
            x - bar_w * 0.5,
            y - r - 18.0,
            bar_w,
            5.0,
            (u.core_hp / u.core_max).clamp(0.0, 1.0),
            Color::new(0.85, 0.55, 0.5, 1.0),
        );
        draw_bar(
            x - bar_w * 0.5,
            y - r - 11.0,
            bar_w,
            4.0,
            u.vigor / u.vigor_max,
            Color::new(0.35, 0.7, 0.4, 1.0),
        );
        draw_bar(
            x - bar_w * 0.5,
            y - r - 5.0,
            bar_w,
            4.0,
            u.strain_frac(),
            Color::new(0.9, 0.55, 0.2, 1.0),
        );

        // Target chevron on the enemy currently in the crosshair.
        if id == self.target && u.side == Side::Enemy && !u.downed {
            draw_triangle(
                vec2(x, y - r - 40.0),
                vec2(x - 8.0, y - r - 52.0),
                vec2(x + 8.0, y - r - 52.0),
                Color::new(0.95, 0.35, 0.3, 1.0),
            );
        }
    }

    fn load_first_weapon(&mut self, data: &GameData, ammo_def: &str) -> bool {
        let Some(id) = self.battle.ridden_unit() else {
            return false;
        };
        let Some(&mount) = self.battle.units[id].weapon_mounts(data).first() else {
            return false;
        };
        self.battle.load_ammo(data, mount, ammo_def)
    }
}

fn draw_bar(x: f32, y: f32, w: f32, h: f32, frac: f32, color: Color) {
    draw_rectangle(x, y, w, h, Color::new(0.12, 0.12, 0.14, 1.0));
    draw_rectangle(x, y, w * frac.clamp(0.0, 1.0), h, color);
}
