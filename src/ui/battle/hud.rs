//! Battle overlay chrome: the top strip, the ridden-creature control panel, the
//! called-shot readout, the combat log, and the pause/outcome banners.

use super::{BattleScreen, GROUND_Y, LOG_LINES};
use crate::combat::{BattleOutcome, CalledTarget};
use crate::data::GameData;
use crate::ui::{LOGICAL_HEIGHT, LOGICAL_WIDTH};
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::draw_ui_text_ex;

impl BattleScreen {
    pub(super) fn draw_hud(&self, data: &GameData) {
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

    pub(super) fn draw_log(&self) {
        let start = self.log.len().saturating_sub(LOG_LINES);
        let mut y = LOGICAL_HEIGHT - 24.0 * LOG_LINES as f32 - 16.0;
        for line in &self.log[start..] {
            draw_ui_text_ex(line, 18.0, y, TextStyle::new(15.0, dark::TEXT_DIM).params());
            y += 24.0;
        }
    }

    pub(super) fn draw_pause_banner(&self) {
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

    pub(super) fn draw_outcome(&self, data: &GameData) {
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
