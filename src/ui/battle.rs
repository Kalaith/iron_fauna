//! The battle screen: side-view battlefield, ridden-creature controls,
//! Wait/Active pacing, and outcome overlay.

use crate::combat::engine::Battle;
use crate::combat::events::BattleEvent;
use crate::combat::{BattleOutcome, CalledTarget, PlayerCommand, Side, Stance, UnitId};
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
    /// Cycled called-shot selection on the current target.
    called: Option<CalledTarget>,
    log: Vec<String>,
    outcome_shown: bool,
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
            log: vec!["The shells close in.".to_owned()],
            outcome_shown: false,
        }
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

    pub fn update(&mut self, data: &GameData, dt: f32) -> BattleScreenResult {
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
            if let Some(line) = describe_event(&self.battle, data, &event) {
                self.log.push(line);
            }
        }
        if self.log.len() > 40 {
            let excess = self.log.len() - 40;
            self.log.drain(..excess);
        }
        BattleScreenResult::Continue
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

        // Target cycling.
        if is_key_pressed(KeyCode::Up) || is_key_pressed(KeyCode::Down) {
            let enemies = self.battle.alive_on(Side::Enemy);
            if !enemies.is_empty() {
                let cur = enemies.iter().position(|&e| e == self.target).unwrap_or(0);
                let next = if is_key_pressed(KeyCode::Up) {
                    (cur + 1) % enemies.len()
                } else {
                    (cur + enemies.len() - 1) % enemies.len()
                };
                self.target = enemies[next];
                self.called = None;
            }
            if self.pace == PaceSetting::Wait {
                self.wait_pause = true; // targeting is thinking time
            }
        }
        if is_key_pressed(KeyCode::C) {
            self.cycle_called();
        }

        // Ridden movement.
        let intent = if is_key_down(KeyCode::Left) {
            -1.0
        } else if is_key_down(KeyCode::Right) {
            1.0
        } else {
            0.0
        };
        let _ = self.battle.command(data, PlayerCommand::Move { intent });

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

    fn cycle_called(&mut self) {
        let Some(target) = self.battle.units.get(self.target) else {
            return;
        };
        // None → each usable mount → each intact limb → None.
        let mounts: Vec<usize> = target
            .mounts
            .iter()
            .enumerate()
            .filter(|(_, m)| m.usable() && target.limbs[m.limb_index].intact())
            .map(|(i, _)| i)
            .collect();
        let limbs = target.intact_limbs();
        let seq: Vec<CalledTarget> = mounts
            .into_iter()
            .map(CalledTarget::Mount)
            .chain(limbs.into_iter().map(CalledTarget::Limb))
            .collect();
        self.called = match self.called {
            None => seq.first().copied(),
            Some(cur) => {
                let idx = seq.iter().position(|&c| c == cur);
                match idx {
                    Some(i) if i + 1 < seq.len() => Some(seq[i + 1]),
                    _ => None,
                }
            }
        };
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
        self.draw_hud(data);
        self.draw_log();
        if self.battle.over() {
            self.draw_outcome(data);
        } else if self.paused() {
            self.draw_pause_banner();
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
        // Big kawaii eye — even in dev-shapes the core should read as alive.
        if !u.downed {
            draw_circle(x + core_r * 0.25, y - core_r * 0.2, core_r * 0.22, BLACK);
            draw_circle(x + core_r * 0.30, y - core_r * 0.26, core_r * 0.08, WHITE);
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
            draw_ui_text_ex(
                "[A] bite  [S] utility  [D] reinforce  [G] regrow  [H] hop  [C] aim  [1-6] stance",
                18.0,
                y + 6.0,
                TextStyle::new(14.0, dark::TEXT_DIM).params(),
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
