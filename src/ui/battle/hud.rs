//! Battle overlay chrome: the top strip, the ridden-creature status line, the
//! target readout, the combat log, and the pause/outcome banners. The command
//! menu itself lives in `menu.rs`.

use super::{BattleScreen, LOG_LINES};
use crate::combat::{BattleOutcome, Side};
use crate::data::GameData;
use crate::ui::{LOGICAL_HEIGHT, LOGICAL_WIDTH};
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::draw_ui_text_ex;

impl BattleScreen {
    pub(super) fn draw_hud(&self, data: &GameData) {
        self.draw_rider_marker(data);

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

        // Ridden status line.
        if let Some(id) = self.battle.ridden_unit() {
            let u = &self.battle.units[id];
            draw_ui_text_ex(
                &format!(
                    "Riding {} — core {:.0}/{:.0}  vigor {:.0}/{:.0}  strain {:.0}/{:.0}",
                    u.spec_name,
                    u.core_hp,
                    u.core_max,
                    u.vigor,
                    u.vigor_max,
                    u.strain,
                    u.strain_threshold
                ),
                18.0,
                50.0,
                TextStyle::new(16.0, dark::TEXT_BRIGHT).params(),
            );
        } else if !self.battle.over() {
            draw_ui_text_ex(
                "RIDER EXPOSED — ride a standing creature",
                18.0,
                50.0,
                TextStyle::new(18.0, Color::new(0.95, 0.4, 0.3, 1.0)).params(),
            );
        }

        // Target readout.
        if let Some(t) = self.battle.units.get(self.target) {
            if t.side == Side::Enemy {
                let state = if t.core_exposed() {
                    "CORE EXPOSED".to_owned()
                } else {
                    format!("{} limbs left", t.intact_limbs().len())
                };
                draw_ui_text_ex(
                    &format!("target: {} — {}", t.spec_name, state),
                    LOGICAL_WIDTH - 340.0,
                    50.0,
                    TextStyle::new(16.0, Color::new(0.95, 0.75, 0.7, 1.0)).params(),
                );
            }
        }
    }

    /// The rider perches on the ridden creature's head (`combat.md` §2.2) — a
    /// glanceable read on who currently has direct control.
    fn draw_rider_marker(&self, data: &GameData) {
        let Some(id) = self.battle.ridden_unit() else {
            return;
        };
        let (x, cy) = self.unit_slot(data, id);
        let r = self.size_radius(data, id);
        let y = cy - r - 18.0;
        draw_circle(x, y - 6.0, 5.0, Color::new(0.9, 0.85, 0.75, 1.0));
        draw_rectangle(
            x - 4.0,
            y - 2.0,
            8.0,
            10.0,
            Color::new(0.55, 0.45, 0.35, 1.0),
        );
    }

    pub(super) fn draw_log(&self) {
        let start = self.log.len().saturating_sub(LOG_LINES);
        let mut y = LOGICAL_HEIGHT - 24.0 * LOG_LINES as f32 - 16.0;
        for line in &self.log[start..] {
            draw_ui_text_ex(
                line,
                LOGICAL_WIDTH - 470.0,
                y,
                TextStyle::new(15.0, dark::TEXT_DIM).params(),
            );
            y += 24.0;
        }
    }

    pub(super) fn draw_pause_banner(&self) {
        draw_rectangle(
            0.0,
            LOGICAL_HEIGHT * 0.5 - 26.0,
            LOGICAL_WIDTH,
            40.0,
            Color::new(0.0, 0.0, 0.0, 0.55),
        );
        draw_ui_text_ex(
            "PAUSED  —  [P] resume",
            LOGICAL_WIDTH * 0.5 - 110.0,
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
