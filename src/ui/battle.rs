//! The battle screen: side-view battlefield, ridden-creature controls,
//! Wait/Active pacing, and outcome overlay.

mod events_view;
mod hud;

use crate::audio::Audio;
use crate::combat::engine::Battle;
use crate::combat::events::BattleEvent;
use crate::combat::{CalledTarget, PlayerCommand, Side, Stance, UnitId};
use crate::data::species::LimbRegion;
use crate::data::GameData;
use crate::state::PaceSetting;
use crate::ui::{creature_art, LOGICAL_HEIGHT, LOGICAL_WIDTH};
use events_view::{describe_event, is_decision_point, sfx_for_event};
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::draw_ui_text_ex;

const GROUND_Y: f32 = 430.0;
const LOG_LINES: usize = 7;

pub enum BattleScreenResult {
    Continue,
    /// Battle resolved and acknowledged — return to the previous state.
    Finished,
}

pub struct BattleScreen {
    pub battle: Battle,
    pub pace: PaceSetting,
    manual_pause: bool,
    wait_pause: bool,
    target: UnitId,
    /// Directional called-shot selection on the current target.
    called: Option<CalledTarget>,
    /// Aim mode: arrows pick the targeted limb region (`combat.md` §3.1).
    aim: bool,
    /// Item mode: number keys use a potion or load ammunition.
    item_mode: bool,
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
            wait_pause: false,
            target,
            called: None,
            aim: false,
            item_mode: false,
            log: vec!["The shells close in.".to_owned()],
            floats: Vec::new(),
            outcome_shown: false,
        }
    }

    /// Test/capture hook: force aim mode and a called shot for a screenshot.
    pub fn force_aim_capture(&mut self, data: &GameData) {
        self.aim = true;
        self.called = self.called_in_regions(data, &[LimbRegion::ArmLeft]);
        self.manual_pause = true;
    }

    /// Test/capture hook: open the item menu for a screenshot.
    pub fn force_item_capture(&mut self) {
        self.item_mode = true;
        self.manual_pause = true;
    }

    fn paused(&self) -> bool {
        self.manual_pause || (self.pace == PaceSetting::Wait && self.wait_pause)
    }

    /// Any confirmed command resumes the clock (`combat.md` §1: Wait pauses
    /// for decisions, never during their resolution).
    fn command(&mut self, data: &GameData, cmd: PlayerCommand) {
        if self.battle.command(data, cmd) {
            self.wait_pause = false;
        }
    }

    pub fn update(&mut self, data: &GameData, audio: &Audio, dt: f32) -> BattleScreenResult {
        if self.battle.over() {
            return self.update_outcome();
        }

        self.handle_input(data);
        if !self.paused() {
            self.battle.update(data, dt);
        }
        for event in self.battle.drain_events() {
            if self.pace == PaceSetting::Wait && is_decision_point(&event) {
                self.wait_pause = true;
            }
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

    /// Turns a battle event into a floating callout over the unit involved.
    fn spawn_float(&mut self, data: &GameData, event: &BattleEvent) {
        let at = |screen: &Self, id: UnitId| -> (f32, f32) {
            let u = &screen.battle.units[id];
            (
                screen.world_to_screen(u.pos, data),
                GROUND_Y - screen.size_radius(data, id) - 30.0,
            )
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

    fn update_outcome(&mut self) -> BattleScreenResult {
        self.outcome_shown = true;
        if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::Space) {
            return BattleScreenResult::Finished;
        }
        BattleScreenResult::Continue
    }

    fn handle_input(&mut self, data: &GameData) {
        if is_key_pressed(KeyCode::P) {
            self.manual_pause = !self.manual_pause;
        }
        if is_key_pressed(KeyCode::Space) && self.paused() {
            self.manual_pause = false;
            self.wait_pause = false;
        }

        // Item mode: [I] toggles; number keys use a potion or load ammo. It
        // swallows the rest of the input so numbers don't also flip stances.
        if is_key_pressed(KeyCode::I) {
            self.item_mode = !self.item_mode;
            if self.item_mode {
                self.aim = false;
                if self.pace == PaceSetting::Wait {
                    self.wait_pause = true;
                }
            }
        }
        if self.item_mode {
            if is_key_pressed(KeyCode::Escape) {
                self.item_mode = false;
            } else {
                self.handle_item_keys(data);
            }
            return;
        }

        // Aim mode: [C] toggles; while aiming, arrows are called-shot region
        // selection and movement is suspended (`combat.md` §3.1).
        if is_key_pressed(KeyCode::C) {
            self.aim = !self.aim;
            if self.aim && self.pace == PaceSetting::Wait {
                self.wait_pause = true; // lining up a shot is thinking time
            }
        }
        // Target cycling (Tab always; Up/Down when not aiming).
        let cycle_fwd = is_key_pressed(KeyCode::Tab) || (!self.aim && is_key_pressed(KeyCode::Up));
        let cycle_back = !self.aim && is_key_pressed(KeyCode::Down);
        if cycle_fwd || cycle_back {
            let enemies = self.battle.alive_on(Side::Enemy);
            if !enemies.is_empty() {
                let cur = enemies.iter().position(|&e| e == self.target).unwrap_or(0);
                let next = if cycle_fwd {
                    (cur + 1) % enemies.len()
                } else {
                    (cur + enemies.len() - 1) % enemies.len()
                };
                self.target = enemies[next];
                self.called = None;
            }
            if self.pace == PaceSetting::Wait {
                self.wait_pause = true;
            }
        }

        if self.aim {
            // Arrows pick the limb region on the target's sprite.
            let region_dir = if is_key_pressed(KeyCode::Up) {
                Some([LimbRegion::Head, LimbRegion::Back].as_slice())
            } else if is_key_pressed(KeyCode::Down) {
                Some([LimbRegion::Legs, LimbRegion::Tail].as_slice())
            } else if is_key_pressed(KeyCode::Left) {
                Some([LimbRegion::ArmLeft].as_slice())
            } else if is_key_pressed(KeyCode::Right) {
                Some([LimbRegion::ArmRight].as_slice())
            } else {
                None
            };
            if let Some(regions) = region_dir {
                self.called = self.called_in_regions(data, regions);
                if self.pace == PaceSetting::Wait {
                    self.wait_pause = true;
                }
            }
            if is_key_pressed(KeyCode::X) {
                self.called = None; // back to center mass
            }
        } else {
            // Ridden movement (only outside aim mode).
            let intent = if is_key_down(KeyCode::Left) {
                -1.0
            } else if is_key_down(KeyCode::Right) {
                1.0
            } else {
                0.0
            };
            let _ = self.battle.command(data, PlayerCommand::Move { intent });
        }

        // Weapons: Q/W/E/R fire the ridden creature's weapon mounts in order.
        let weapon_keys = [KeyCode::Q, KeyCode::W, KeyCode::E, KeyCode::R];
        if let Some(id) = self.battle.ridden_unit() {
            let mounts = self.battle.units[id].weapon_mounts(data);
            for (slot, key) in weapon_keys.iter().enumerate() {
                if is_key_pressed(*key) {
                    if let Some(&mount) = mounts.get(slot) {
                        self.command(
                            data,
                            PlayerCommand::Fire {
                                mount,
                                target: self.target,
                                called: self.called,
                            },
                        );
                    }
                }
            }
        }
        if is_key_pressed(KeyCode::A) {
            self.command(
                data,
                PlayerCommand::NaturalAttack {
                    target: self.target,
                    called: self.called,
                },
            );
        }
        if is_key_pressed(KeyCode::S) {
            self.trigger_first_utility(data);
        }
        if is_key_pressed(KeyCode::D) {
            self.command(data, PlayerCommand::Reinforce);
        }
        if is_key_pressed(KeyCode::G) {
            if let Some(id) = self.battle.ridden_unit() {
                let severed = self.battle.units[id].limbs.iter().position(|l| l.severed);
                if let Some(limb) = severed {
                    self.command(data, PlayerCommand::Regrow { limb });
                }
            }
        }
        if is_key_pressed(KeyCode::H) {
            self.hop_next(data);
        }
        // Number keys toggle stances party-wide.
        let stance_keys = [
            KeyCode::Key1,
            KeyCode::Key2,
            KeyCode::Key3,
            KeyCode::Key4,
            KeyCode::Key5,
            KeyCode::Key6,
        ];
        for (i, key) in stance_keys.iter().enumerate() {
            if is_key_pressed(*key) {
                let friendlies = self.battle.alive_on(Side::Player);
                if let Some(&unit) = friendlies.get(i) {
                    let stance = match self.battle.units[unit].stance {
                        Stance::Aggressive => Stance::Defensive,
                        Stance::Defensive => Stance::Aggressive,
                    };
                    self.command(data, PlayerCommand::SetStance { unit, stance });
                }
            }
        }
    }

    /// In item mode, number keys pick from potions first, then ammo. Using a
    /// potion is instant; loading ammo goes into the ridden unit's first weapon
    /// (a reload that occupies it). Either resumes a Wait-paused clock.
    fn handle_item_keys(&mut self, data: &GameData) {
        let potions = self.battle.usable_potions(data);
        let ammo = self.battle.usable_ammo(data);
        let keys = [
            KeyCode::Key1,
            KeyCode::Key2,
            KeyCode::Key3,
            KeyCode::Key4,
            KeyCode::Key5,
            KeyCode::Key6,
            KeyCode::Key7,
            KeyCode::Key8,
        ];
        for (i, key) in keys.iter().enumerate() {
            if !is_key_pressed(*key) {
                continue;
            }
            let used = if i < potions.len() {
                self.battle.use_potion(data, &potions[i].0.clone())
            } else if let Some((def_id, _)) = ammo.get(i - potions.len()).cloned() {
                self.load_first_weapon(data, &def_id)
            } else {
                false
            };
            if used {
                self.wait_pause = false;
                self.item_mode = false;
            }
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

    /// Picks a called-shot target in one of the given sprite regions: prefer
    /// a still-usable weapon/graft mount there (surgery), else the bare limb.
    fn called_in_regions(&self, data: &GameData, regions: &[LimbRegion]) -> Option<CalledTarget> {
        let target = self.battle.units.get(self.target)?;
        let in_region = |limb_index: usize| {
            let region = target.limb_def(data, &target.limbs[limb_index]).region;
            regions.contains(&region)
        };
        // A live mount on a limb in-region is the most valuable pick.
        if let Some(mi) = target.mounts.iter().position(|m| {
            m.usable() && target.limbs[m.limb_index].intact() && in_region(m.limb_index)
        }) {
            return Some(CalledTarget::Mount(mi));
        }
        // Otherwise sever an intact limb in-region.
        target
            .intact_limbs()
            .into_iter()
            .find(|&li| in_region(li))
            .map(CalledTarget::Limb)
    }

    fn trigger_first_utility(&mut self, data: &GameData) {
        let Some(id) = self.battle.ridden_unit() else {
            return;
        };
        let u = &self.battle.units[id];
        let candidate = u.mounts.iter().enumerate().find(|(_, m)| {
            m.usable()
                && u.limbs[m.limb_index].intact()
                && m.cooldown <= 0.0
                && data
                    .graftware
                    .get(&m.def_id)
                    .and_then(|d| d.effect)
                    .is_some()
        });
        if let Some((mount, _)) = candidate {
            self.command(data, PlayerCommand::TriggerUtility { mount, ally: None });
        }
    }

    fn hop_next(&mut self, data: &GameData) {
        let friendlies = self.battle.alive_on(Side::Player);
        if friendlies.len() < 2 && self.battle.ridden_unit().is_some() {
            return;
        }
        let current = self.battle.ridden_unit();
        let next = match current {
            Some(cur) => {
                let idx = friendlies.iter().position(|&u| u == cur).unwrap_or(0);
                friendlies.get((idx + 1) % friendlies.len()).copied()
            }
            None => friendlies.first().copied(),
        };
        if let Some(to) = next {
            if Some(to) != current {
                self.command(data, PlayerCommand::BeginHop { to });
            }
        }
    }

    // ------------------------------------------------------------------
    // Drawing
    // ------------------------------------------------------------------

    pub fn draw(&self, data: &GameData) {
        self.draw_backdrop();
        for (id, _) in self.battle.units.iter().enumerate() {
            self.draw_unit(data, id);
        }
        if self.aim {
            self.draw_aim_overlay(data);
        }
        self.draw_floats();
        self.draw_hud(data);
        if self.item_mode {
            self.draw_item_menu(data);
        }
        self.draw_log();
        if self.battle.over() {
            self.draw_outcome(data);
        } else if self.paused() {
            self.draw_pause_banner();
        }
    }

    /// Directional called-shot overlay on the current target: chevrons around
    /// the sprite mapped to limb regions, the chosen one lit (`combat.md` §3.1).
    fn draw_aim_overlay(&self, data: &GameData) {
        let Some(target) = self.battle.units.get(self.target) else {
            return;
        };
        if target.downed {
            return;
        }
        let x = self.world_to_screen(target.pos, data);
        let r = self.size_radius(data, self.target);
        let y = GROUND_Y - r;

        // Which region is currently selected, for highlighting.
        let sel_region = match self.called {
            Some(CalledTarget::Mount(mi)) => target
                .mounts
                .get(mi)
                .map(|m| target.limb_def(data, &target.limbs[m.limb_index]).region),
            Some(CalledTarget::Limb(li)) => target
                .limbs
                .get(li)
                .map(|l| target.limb_def(data, l).region),
            None => None,
        };
        let regs = [
            (LimbRegion::Head, 0.0, -1.0, "▲ head/back"),
            (LimbRegion::Legs, 0.0, 1.0, "▼ legs"),
            (LimbRegion::ArmLeft, -1.0, 0.0, "◀ left"),
            (LimbRegion::ArmRight, 1.0, 0.0, "▶ right"),
        ];
        let matches = |sel: Option<LimbRegion>, r: LimbRegion| match (sel, r) {
            (Some(LimbRegion::Head), LimbRegion::Head)
            | (Some(LimbRegion::Back), LimbRegion::Head) => true,
            (Some(LimbRegion::Legs), LimbRegion::Legs)
            | (Some(LimbRegion::Tail), LimbRegion::Legs) => true,
            (Some(s), r) => s == r,
            _ => false,
        };
        for (region, dx, dy, _label) in regs {
            let lit = matches(sel_region, region);
            let cx = x + dx * (r + 26.0);
            let cy = y + dy * (r + 22.0);
            let col = if lit {
                Color::new(0.98, 0.45, 0.35, 1.0)
            } else {
                Color::new(0.6, 0.6, 0.65, 0.5)
            };
            draw_triangle(
                vec2(cx + dx * 8.0, cy + dy * 8.0),
                vec2(cx - dy * 6.0 - dx * 4.0, cy - dx * 6.0 - dy * 4.0),
                vec2(cx + dy * 6.0 - dx * 4.0, cy + dx * 6.0 - dy * 4.0),
                col,
            );
        }
        draw_circle_lines(x, y, r + 14.0, 2.0, Color::new(0.98, 0.45, 0.35, 0.7));
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

    fn world_to_screen(&self, pos: f32, data: &GameData) -> f32 {
        let half = data.balance.battle.arena_half_width;
        LOGICAL_WIDTH * 0.5 + pos / half * (LOGICAL_WIDTH * 0.5 - 80.0)
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
        use crate::data::species::SizeClass;
        match self.battle.units[id].species(data).size {
            SizeClass::Small => 20.0,
            SizeClass::Medium => 30.0,
            SizeClass::Large => 42.0,
            SizeClass::Huge => 54.0,
        }
    }

    fn draw_unit(&self, data: &GameData, id: UnitId) {
        let u = &self.battle.units[id];
        let x = self.world_to_screen(u.pos, data);
        let r = self.size_radius(data, id);
        // Body centre sits a radius above the ground so the feet meet the line.
        let y = GROUND_Y - r;
        let species = u.species(data);

        // Grafts still riding on intact limbs, as the procedural renderer draws
        // them elsewhere (bench, bestiary) — one shared creature look.
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
        let bar_w = r * 2.0;
        draw_bar(
            x - r,
            y - r - 18.0,
            bar_w,
            5.0,
            u.vigor / u.vigor_max,
            Color::new(0.35, 0.7, 0.4, 1.0),
        );
        draw_bar(
            x - r,
            y - r - 10.0,
            bar_w,
            5.0,
            u.strain_frac(),
            Color::new(0.9, 0.55, 0.2, 1.0),
        );

        // Target chevron.
        if id == self.target && !u.downed {
            draw_triangle(
                vec2(x, y - r - 40.0),
                vec2(x - 8.0, y - r - 52.0),
                vec2(x + 8.0, y - r - 52.0),
                Color::new(0.95, 0.35, 0.3, 1.0),
            );
        }
    }
}

fn draw_bar(x: f32, y: f32, w: f32, h: f32, frac: f32, color: Color) {
    draw_rectangle(x, y, w, h, Color::new(0.12, 0.12, 0.14, 1.0));
    draw_rectangle(x, y, w * frac.clamp(0.0, 1.0), h, color);
}
