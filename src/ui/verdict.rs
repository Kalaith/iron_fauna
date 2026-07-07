//! The factory verdict screen (`game_design.md` §9): Purge, Reseed, or Bind —
//! and the grow-core facility of a Bound Gestarium.

use crate::data::GameData;
use crate::model::worldstate::Verdict;
use crate::state::GameSession;
use crate::ui::{element_color, menu_button, LOGICAL_HEIGHT, LOGICAL_WIDTH};
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::{draw_ui_text_ex, RectExt, VirtualUi};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerdictAction {
    Choose(Verdict),
    GrowCore(String),
    Close,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactoryScreenKind {
    /// The heart is silenced; the region's fate is in your hands.
    Verdict,
    /// A Bound factory's gestation menu.
    Grow,
}

pub struct VerdictScreen {
    pub factory_id: String,
    pub kind: FactoryScreenKind,
}

impl VerdictScreen {
    pub fn draw(
        &self,
        data: &GameData,
        session: &GameSession,
        ui: &VirtualUi,
    ) -> Vec<VerdictAction> {
        match self.kind {
            FactoryScreenKind::Verdict => self.draw_verdict(data, ui),
            FactoryScreenKind::Grow => self.draw_grow(data, session, ui),
        }
    }

    fn draw_verdict(&self, data: &GameData, ui: &VirtualUi) -> Vec<VerdictAction> {
        let mut actions = Vec::new();
        let mouse = ui.mouse_position();
        let Some(factory) = data.factories.get(&self.factory_id) else {
            actions.push(VerdictAction::Close);
            return actions;
        };

        draw_rectangle(
            0.0,
            0.0,
            LOGICAL_WIDTH,
            LOGICAL_HEIGHT,
            Color::new(0.03, 0.02, 0.03, 1.0),
        );
        draw_ui_text_ex(
            &format!("{} LIES SILENT", factory.name.to_uppercase()),
            90.0,
            110.0,
            TextStyle::new(38.0, Color::new(0.88, 0.82, 0.75, 1.0)).params(),
        );
        draw_text_block(
            "The vats have stopped. Whatever you decide here, the region will carry it forever. There is no one else to ask.",
            92.0,
            130.0,
            900.0,
            60.0,
            18.0,
            6.0,
            dark::TEXT_DIM,
        );

        let choices: [(Verdict, &str, &str, Color); 3] = [
            (
                Verdict::Purge,
                "PURGE",
                "Burn the wombs. No more war-units will ever crawl out of this place — and nothing else will, either. The region stays safe, scarred, and dead.",
                Color::new(0.65, 0.30, 0.25, 1.0),
            ),
            (
                Verdict::Reseed,
                "RESEED",
                "Restore its first purpose. Dead ground blooms, water clears, gentle cores return. But a living factory is a loaded gun, and someone will eventually pick it up.",
                Color::new(0.35, 0.60, 0.38, 1.0),
            ),
            (
                Verdict::Bind,
                "BIND",
                "Claim it. The machine answers to you now: grow your own cores in its wombs. You become the thing the Progenitors were, and tell yourself it's different.",
                Color::new(0.45, 0.40, 0.65, 1.0),
            ),
        ];

        for (i, (verdict, title, desc, accent)) in choices.iter().enumerate() {
            let rect = Rect::new(90.0, 230.0 + i as f32 * 140.0, LOGICAL_WIDTH - 180.0, 120.0);
            let hovered = rect.contains_point(mouse);
            draw_surface(
                rect,
                &SurfaceStyle::new(if hovered {
                    Color::new(0.13, 0.13, 0.16, 1.0)
                } else {
                    Color::new(0.08, 0.08, 0.10, 1.0)
                })
                .with_border(1.0, Color::new(accent.r, accent.g, accent.b, 0.7))
                .with_left_accent(5.0, *accent),
            );
            draw_ui_text_ex(
                title,
                rect.x + 22.0,
                rect.y + 36.0,
                TextStyle::new(24.0, *accent).params(),
            );
            draw_text_block(
                desc,
                rect.x + 22.0,
                rect.y + 48.0,
                rect.w - 44.0,
                60.0,
                15.0,
                5.0,
                dark::TEXT,
            );
            if hovered && is_mouse_button_released(MouseButton::Left) {
                actions.push(VerdictAction::Choose(*verdict));
            }
        }
        actions
    }

    fn draw_grow(
        &self,
        data: &GameData,
        session: &GameSession,
        ui: &VirtualUi,
    ) -> Vec<VerdictAction> {
        let mut actions = Vec::new();
        let mouse = ui.mouse_position();
        let Some(factory) = data.factories.get(&self.factory_id) else {
            actions.push(VerdictAction::Close);
            return actions;
        };
        if is_key_pressed(KeyCode::Escape) {
            actions.push(VerdictAction::Close);
        }

        draw_rectangle(
            0.0,
            0.0,
            LOGICAL_WIDTH,
            LOGICAL_HEIGHT,
            Color::new(0.04, 0.03, 0.05, 0.92),
        );
        let rect = Rect::new(240.0, 90.0, 800.0, 540.0);
        draw_surface_with_title(
            rect,
            Some(&format!(
                "{} — Bound. The wombs are yours now.",
                factory.name
            )),
            &SurfaceStyle::new(Color::new(0.08, 0.075, 0.10, 0.98))
                .with_border(1.0, Color::new(0.45, 0.40, 0.65, 0.8))
                .with_header(38.0, Color::new(0.11, 0.10, 0.14, 1.0))
                .with_header_divider(1.0, Color::new(0.45, 0.40, 0.65, 0.5)),
            TextStyle::new(16.0, dark::TEXT),
        );
        let content = rect.inset(18.0);
        let mut y = content.y + 46.0;
        draw_ui_text_ex(
            &format!(
                "Gestating a core costs {} scrip. You have {}.",
                factory.grow_cost, session.profile.inventory.scrip
            ),
            content.x,
            y,
            TextStyle::new(15.0, dark::TEXT_DIM).params(),
        );
        y += 30.0;

        for species_id in &factory.grows {
            let Some(species) = data.species.get(species_id) else {
                continue;
            };
            let row = Rect::new(content.x, y, content.w, 52.0);
            draw_surface(row, &SurfaceStyle::new(Color::new(0.10, 0.10, 0.14, 1.0)));
            draw_circle(
                row.x + 20.0,
                row.y + 26.0,
                10.0,
                element_color(species.element),
            );
            draw_ui_text_ex(
                &format!(
                    "{} — {} {} · Power {} · Speed {}",
                    species.name,
                    species.size.display_name(),
                    species.archetype.display_name(),
                    species.power,
                    species.speed
                ),
                row.x + 42.0,
                row.y + 22.0,
                TextStyle::new(15.0, dark::TEXT).params(),
            );
            draw_ui_text_ex(
                &format!(
                    "{} · {}",
                    species.element.display_name(),
                    species.temperament.display_name()
                ),
                row.x + 42.0,
                row.y + 42.0,
                TextStyle::new(12.0, dark::TEXT_DIM).params(),
            );
            if menu_button(
                Rect::new(row.right() - 110.0, row.y + 9.0, 102.0, 34.0),
                "Grow",
                session.profile.inventory.scrip >= factory.grow_cost,
                mouse,
            ) {
                actions.push(VerdictAction::GrowCore(species_id.clone()));
            }
            y += 58.0;
        }

        if menu_button(
            Rect::new(content.x, content.bottom() - 42.0, 130.0, 36.0),
            "Step away",
            true,
            mouse,
        ) {
            actions.push(VerdictAction::Close);
        }
        actions
    }
}
