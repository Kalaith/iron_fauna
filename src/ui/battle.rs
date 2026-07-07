//! The battle screen: side-view battlefield, ridden-creature controls,
//! Wait/Active pacing, and outcome overlay.

use crate::audio::{Audio, Sfx};
use crate::combat::engine::Battle;
use crate::combat::events::BattleEvent;
use crate::combat::{BattleOutcome, CalledTarget, PlayerCommand, Side, Stance, UnitId};
use crate::data::species::LimbRegion;
use crate::data::GameData;
use crate::state::PaceSetting;
use crate::ui::{element_color, LOGICAL_HEIGHT, LOGICAL_WIDTH};
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
        let y = GROUND_Y - r;
        let species = u.species(data);
        let body_color = if u.downed {
            Color::new(0.25, 0.25, 0.28, 1.0)
        } else {
            element_color(species.element)
        };

        // War-body shell: intact limbs drawn as plates around the core.
        let limb_count = u.limbs.len().max(1);
        for (i, limb) in u.limbs.iter().enumerate() {
            let angle = std::f32::consts::TAU * (i as f32 / limb_count as f32) - 1.2;
            let lx = x + angle.cos() * r * 0.95;
            let ly = y + angle.sin() * r * 0.7;
            if limb.intact() {
                let frac = (limb.hp / limb.max_hp).clamp(0.0, 1.0);
                let c = Color::new(
                    body_color.r * (0.5 + 0.5 * frac),
                    body_color.g * (0.5 + 0.5 * frac),
                    body_color.b * (0.5 + 0.5 * frac),
                    1.0,
                );
                draw_rectangle(lx - 6.0, ly - 6.0, 12.0, 12.0, c);
                draw_rectangle_lines(
                    lx - 6.0,
                    ly - 6.0,
                    12.0,
                    12.0,
                    1.5,
                    Color::new(0.1, 0.1, 0.1, 0.8),
                );
            } else {
                // Severed: a raw stump, regrowth shown filling back in.
                let regrow = self.battle.units[id].regrow_target == Some(i);
                let frac = (limb.regrow_hp / limb.max_hp).clamp(0.0, 1.0);
                draw_rectangle(
                    lx - 5.0,
                    ly - 5.0,
                    10.0,
                    10.0,
                    Color::new(0.35, 0.12, 0.12, 0.9),
                );
                if regrow || frac > 0.0 {
                    draw_rectangle(
                        lx - 5.0,
                        ly - 5.0 + 10.0 * (1.0 - frac),
                        10.0,
                        10.0 * frac,
                        Color::new(0.4, 0.6, 0.35, 0.9),
                    );
                }
            }
            // Mounted graftware on this limb: ugly bolt-on triangles.
            for m in u.mounts.iter().filter(|m| m.limb_index == i) {
                let mc = if m.destroyed {
                    Color::new(0.5, 0.15, 0.1, 1.0)
                } else if m.detached {
                    Color::new(0.3, 0.3, 0.3, 0.6)
                } else {
                    Color::new(0.75, 0.72, 0.65, 1.0)
                };
                draw_triangle(
                    vec2(lx, ly - 12.0),
                    vec2(lx - 5.0, ly - 4.0),
                    vec2(lx + 5.0, ly - 4.0),
                    mc,
                );
            }
        }

        // The core: the small precious center. Exposed cores glow.
        let core_frac = (u.core_hp / u.core_max).clamp(0.0, 1.0);
        let exposed = u.core_exposed();
        let core_r = r * 0.45;
        if exposed && !u.downed {
            draw_circle(x, y, core_r + 4.0, Color::new(1.0, 0.9, 0.6, 0.35));
        }
        draw_circle(x, y, core_r, Color::new(0.95, 0.92, 0.88, 1.0));
        draw_circle(
            x,
            y,
            core_r * core_frac.max(0.15),
            if u.downed {
                Color::new(0.4, 0.4, 0.45, 1.0)
            } else {
                body_color
            },
        );
        // Kawaii face: two big eyes, catch-lights, and a blush — the core is
        // the precious thing you must protect on sight (`game_design.md` §10).
        if !u.downed {
            let face = x + if u.side == Side::Player { -1.0 } else { 1.0 } * core_r * 0.12;
            let eye_dx = core_r * 0.34;
            let eye_y = y - core_r * 0.12;
            let eye_r = core_r * 0.24;
            for side in [-1.0, 1.0] {
                let ex = face + side * eye_dx;
                draw_circle(ex, eye_y, eye_r, Color::new(0.1, 0.1, 0.12, 1.0));
                draw_circle(ex + eye_r * 0.3, eye_y - eye_r * 0.35, eye_r * 0.4, WHITE);
                // Soft blush under each eye.
                draw_circle(
                    ex,
                    eye_y + eye_r * 1.3,
                    eye_r * 0.5,
                    Color::new(0.95, 0.55, 0.55, 0.35),
                );
            }
            // Tiny mouth.
            draw_circle(
                face,
                y + core_r * 0.42,
                core_r * 0.07,
                Color::new(0.4, 0.2, 0.22, 0.9),
            );
        }

        // Shield ring.
        if u.shield > 0.0 {
            draw_circle_lines(x, y, r + 6.0, 2.0, Color::new(0.5, 0.75, 0.95, 0.8));
        }
        // Berserk flare.
        if u.berserk() {
            draw_circle_lines(x, y, r + 10.0, 2.5, Color::new(0.95, 0.3, 0.2, 0.9));
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

    fn draw_hud(&self, data: &GameData) {
        // Rider sits on the ridden creature's head (`combat.md` §2.2).
        if let Some(id) = self.battle.ridden_unit() {
            let u = &self.battle.units[id];
            let x = self.world_to_screen(u.pos, data);
            let r = self.size_radius(data, id);
            let y = GROUND_Y - r * 2.0 - 8.0;
            draw_circle(x, y - 6.0, 5.0, Color::new(0.9, 0.85, 0.75, 1.0));
            draw_rectangle(
                x - 4.0,
                y - 2.0,
                8.0,
                10.0,
                Color::new(0.55, 0.45, 0.35, 1.0),
            );
        } else if let Some((to, timer)) = self.battle.rider.hop {
            let u = &self.battle.units[to];
            let x = self.world_to_screen(u.pos, data);
            draw_ui_text_ex(
                &format!("rider crossing… {:.1}s", timer),
                x - 50.0,
                GROUND_Y - 120.0,
                TextStyle::new(14.0, Color::new(0.95, 0.85, 0.5, 1.0)).params(),
            );
        } else if self.battle.rider.exposed() && !self.battle.over() {
            draw_ui_text_ex(
                "RIDER EXPOSED — hop to a standing creature [H]",
                LOGICAL_WIDTH * 0.5 - 220.0,
                80.0,
                TextStyle::new(20.0, Color::new(0.95, 0.4, 0.3, 1.0)).params(),
            );
        }

        // Top strip: context + clock + pace.
        let context = match self.battle.context {
            crate::combat::BattleContext::WildSubdue => "Wild Subdue",
            crate::combat::BattleContext::FactoryDismantle => "Factory Dismantle",
            crate::combat::BattleContext::Duel => "Sanctioned Duel",
        };
        draw_ui_text_ex(
            &format!(
                "{}   ·   {:.0}s   ·   pace: {}   [P] pause",
                context,
                self.battle.time,
                self.pace.display_name()
            ),
            18.0,
            26.0,
            TextStyle::new(16.0, dark::TEXT_DIM).params(),
        );

        // Ridden panel: weapons + called shot.
        if let Some(id) = self.battle.ridden_unit() {
            let u = &self.battle.units[id];
            let mut y = 52.0;
            draw_ui_text_ex(
                &format!(
                    "Riding {} — vigor {:.0}/{:.0}  strain {:.0}/{:.0}",
                    u.spec_name, u.vigor, u.vigor_max, u.strain, u.strain_threshold
                ),
                18.0,
                y,
                TextStyle::new(16.0, dark::TEXT_BRIGHT).params(),
            );
            y += 24.0;
            let keys = ["Q", "W", "E", "R"];
            for (i, &mi) in u.weapon_mounts(data).iter().take(4).enumerate() {
                let m = &u.mounts[mi];
                let def_name = data
                    .graftware
                    .get(&m.def_id)
                    .map(|d| d.name.as_str())
                    .unwrap_or("?");
                let status = if m.cooldown > 0.0 {
                    format!("{:.1}s", m.cooldown)
                } else {
                    "ready".to_owned()
                };
                draw_ui_text_ex(
                    &format!("[{}] {} — {}", keys[i], def_name, status),
                    18.0,
                    y,
                    TextStyle::new(
                        15.0,
                        if m.cooldown <= 0.0 {
                            dark::TEXT
                        } else {
                            dark::TEXT_DIM
                        },
                    )
                    .params(),
                );
                y += 20.0;
            }
            let hint = if self.aim {
                "AIMING — arrows pick a limb · Q/W/E/R fire · [X] center · [C] release · [Tab] switch foe"
            } else {
                "[A] bite  [S] utility  [D] reinforce  [G] regrow  [H] hop  [C] aim  [1-6] stance"
            };
            draw_ui_text_ex(
                hint,
                18.0,
                y + 6.0,
                TextStyle::new(
                    14.0,
                    if self.aim {
                        Color::new(0.98, 0.55, 0.4, 1.0)
                    } else {
                        dark::TEXT_DIM
                    },
                )
                .params(),
            );
        }

        // Called-shot readout on the target.
        if let Some(target) = self.battle.units.get(self.target) {
            let called_desc = match self.called {
                None => "center mass".to_owned(),
                Some(CalledTarget::Mount(mi)) => match target.mounts.get(mi) {
                    Some(m) => format!(
                        "aimed: {} ({:.0}%)",
                        data.graftware
                            .get(&m.def_id)
                            .map(|d| d.name.as_str())
                            .unwrap_or("graft"),
                        (m.graft_hp / m.graft_hp_max * 100.0).clamp(0.0, 100.0)
                    ),
                    None => "aimed: graft".to_owned(),
                },
                Some(CalledTarget::Limb(li)) => format!(
                    "aimed: {}",
                    target
                        .limbs
                        .get(li)
                        .map(|l| target.limb_def(data, l).name.clone())
                        .unwrap_or_else(|| "limb".to_owned())
                ),
            };
            draw_ui_text_ex(
                &format!("target: {} — {}", target.spec_name, called_desc),
                LOGICAL_WIDTH - 420.0,
                26.0,
                TextStyle::new(16.0, Color::new(0.95, 0.75, 0.7, 1.0)).params(),
            );
        }
    }

    fn draw_log(&self) {
        let start = self.log.len().saturating_sub(LOG_LINES);
        let mut y = LOGICAL_HEIGHT - 24.0 * LOG_LINES as f32 - 16.0;
        for line in &self.log[start..] {
            draw_ui_text_ex(line, 18.0, y, TextStyle::new(15.0, dark::TEXT_DIM).params());
            y += 24.0;
        }
    }

    fn draw_pause_banner(&self) {
        let label = if self.manual_pause {
            "PAUSED  —  [P] resume"
        } else {
            "WAITING — choose an action, or [Space] to let it ride"
        };
        draw_rectangle(
            0.0,
            LOGICAL_HEIGHT * 0.5 - 26.0,
            LOGICAL_WIDTH,
            40.0,
            Color::new(0.0, 0.0, 0.0, 0.55),
        );
        draw_ui_text_ex(
            label,
            LOGICAL_WIDTH * 0.5 - 200.0,
            LOGICAL_HEIGHT * 0.5,
            TextStyle::new(20.0, Color::new(0.9, 0.9, 0.85, 1.0)).params(),
        );
    }

    fn draw_outcome(&self, data: &GameData) {
        draw_rectangle(
            0.0,
            0.0,
            LOGICAL_WIDTH,
            LOGICAL_HEIGHT,
            Color::new(0.0, 0.0, 0.0, 0.65),
        );
        let (title, lines) = match &self.battle.outcome {
            Some(BattleOutcome::Victory(rewards)) => {
                let mut lines = Vec::new();
                for s in &rewards.captured_species {
                    let name = data.species.get(s).map(|d| d.name.as_str()).unwrap_or(s);
                    lines.push(format!("core recovered: {}", name));
                }
                for s in &rewards.salvage {
                    let name = data.graftware.get(s).map(|d| d.name.as_str()).unwrap_or(s);
                    lines.push(format!("salvage: {}", name));
                }
                lines.push(format!("scrip: +{}", rewards.scrip));
                ("THE SHELL COMES DOWN", lines)
            }
            Some(BattleOutcome::Fled) => (
                "YOU FLEE AND ESCAPE",
                vec!["Every core cracked. They will recover — so will you.".to_owned()],
            ),
            None => ("", Vec::new()),
        };
        draw_ui_text_ex(
            title,
            LOGICAL_WIDTH * 0.5 - 190.0,
            240.0,
            TextStyle::new(34.0, Color::new(0.92, 0.9, 0.85, 1.0)).params(),
        );
        let mut y = 300.0;
        for line in &lines {
            draw_ui_text_ex(
                line,
                LOGICAL_WIDTH * 0.5 - 160.0,
                y,
                TextStyle::new(18.0, dark::TEXT).params(),
            );
            y += 28.0;
        }
        draw_ui_text_ex(
            "[Enter] continue",
            LOGICAL_WIDTH * 0.5 - 70.0,
            y + 30.0,
            TextStyle::new(16.0, dark::TEXT_DIM).params(),
        );
    }
}

fn draw_bar(x: f32, y: f32, w: f32, h: f32, frac: f32, color: Color) {
    draw_rectangle(x, y, w, h, Color::new(0.12, 0.12, 0.14, 1.0));
    draw_rectangle(x, y, w * frac.clamp(0.0, 1.0), h, color);
}

/// Maps a battle event to its sound effect, if any.
fn sfx_for_event(event: &BattleEvent) -> Option<Sfx> {
    Some(match event {
        BattleEvent::Hit { to_core, .. } => {
            if *to_core {
                Sfx::Crack
            } else {
                Sfx::Hit
            }
        }
        BattleEvent::LimbSevered { .. } => Sfx::Sever,
        BattleEvent::CoreCracked { .. } => Sfx::Crack,
        BattleEvent::HopStarted { .. } => Sfx::Hop,
        BattleEvent::GraftRejected { .. } | BattleEvent::BerserkStarted { .. } => Sfx::Reject,
        _ => return None,
    })
}

fn is_decision_point(event: &BattleEvent) -> bool {
    matches!(
        event,
        BattleEvent::RiddenActionReady { .. }
            | BattleEvent::HopLanded { .. }
            | BattleEvent::RiderExposed
            | BattleEvent::BerserkStarted { .. }
            | BattleEvent::CoreExposed { .. }
    )
}

fn describe_event(battle: &Battle, data: &GameData, event: &BattleEvent) -> Option<String> {
    let name = |id: &UnitId| battle.units[*id].spec_name.clone();
    let line = match event {
        BattleEvent::Hit {
            attacker,
            target,
            amount,
            to_core,
        } => {
            if *to_core {
                format!(
                    "{} strikes {}'s core for {:.0}!",
                    name(attacker),
                    name(target),
                    amount
                )
            } else {
                return None; // routine limb chip — too noisy for the log
            }
        }
        BattleEvent::Miss { attacker, target } => {
            format!("{} misses {}", name(attacker), name(target))
        }
        BattleEvent::LimbSevered { unit, limb_name } => {
            format!("{} loses its {}!", name(unit), limb_name)
        }
        BattleEvent::LimbRegrown { unit, limb_name } => {
            format!("{}'s {} regrows", name(unit), limb_name)
        }
        BattleEvent::GraftDestroyed { unit, graft_name } => {
            format!("{}'s {} is destroyed", name(unit), graft_name)
        }
        BattleEvent::GraftRejected { unit, graft_name } => {
            format!("{} REJECTS its {}!", name(unit), graft_name)
        }
        BattleEvent::SalvageDropped { def_id } => {
            let n = data
                .graftware
                .get(def_id)
                .map(|d| d.name.clone())
                .unwrap_or_else(|| def_id.clone());
            format!("{} clatters to the ground", n)
        }
        BattleEvent::CoreExposed { unit } => format!("{}'s core is EXPOSED", name(unit)),
        BattleEvent::CoreCracked { unit } => {
            format!("{}'s core cracks — it's over for it", name(unit))
        }
        BattleEvent::BerserkStarted { unit } => format!("{} goes BERSERK!", name(unit)),
        BattleEvent::BerserkEnded { unit } => format!("{} calms", name(unit)),
        BattleEvent::HopStarted { to, .. } => format!("rider leaps toward {}", name(to)),
        BattleEvent::HopLanded { to } => format!("rider mounts {}", name(to)),
        BattleEvent::RiderExposed => "the rider is thrown into the open!".to_owned(),
        BattleEvent::Healed { target, amount, .. } => {
            format!("{} is soothed for {:.0}", name(target), amount)
        }
        BattleEvent::Shielded { unit, amount } => {
            format!("{}'s core is shielded (+{:.0})", name(unit), amount)
        }
        BattleEvent::StanceChanged { unit } => format!(
            "{} switches to {}",
            name(unit),
            battle.units[*unit].stance.display_name()
        ),
        BattleEvent::RiddenActionReady { .. } => return None,
        BattleEvent::BattleEnded => return None,
    };
    Some(line)
}
