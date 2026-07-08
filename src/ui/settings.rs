//! Settings — options reachable from the title screen. Pure view layer: it
//! reads state and returns intents; `game.rs` applies them. Save/Load/Exit
//! live in the in-game Codex, not here — this screen is the title-side options.

use crate::state::{GameSession, PaceSetting};
use crate::ui::{menu_button, LOGICAL_HEIGHT, LOGICAL_WIDTH};
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::{draw_ui_text_ex, VirtualUi};

pub struct SettingsScreen;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsAction {
    Back,
    TogglePace,
}

impl SettingsScreen {
    pub fn draw(&self, session: &GameSession, ui: &VirtualUi) -> Vec<SettingsAction> {
        let mut actions = Vec::new();
        let mouse = ui.mouse_position();
        clear_background(Color::new(0.05, 0.055, 0.07, 1.0));

        if is_key_pressed(KeyCode::Escape) {
            actions.push(SettingsAction::Back);
        }

        draw_ui_text_ex(
            "SETTINGS",
            72.0,
            88.0,
            TextStyle::new(38.0, Color::new(0.88, 0.86, 0.80, 1.0)).params(),
        );

        // Combat pace — whether battles wait on your input or run in real time.
        let row = Rect::new(72.0, 150.0, LOGICAL_WIDTH - 144.0, 84.0);
        draw_surface(
            row,
            &SurfaceStyle::new(Color::new(0.08, 0.085, 0.11, 0.95))
                .with_border(1.0, Color::new(0.30, 0.34, 0.42, 0.6)),
        );
        draw_ui_text_ex(
            "Combat Pace",
            row.x + 20.0,
            row.y + 30.0,
            TextStyle::new(21.0, dark::TEXT_BRIGHT).params(),
        );
        draw_ui_text_ex(
            match session.pace {
                PaceSetting::Wait => "Battles pause for your orders each turn.",
                PaceSetting::Active => "Battles flow in real time; act when ready.",
            },
            row.x + 20.0,
            row.y + 58.0,
            TextStyle::new(15.0, dark::TEXT_DIM).params(),
        );
        if menu_button(
            Rect::new(row.right() - 200.0, row.y + 22.0, 180.0, 40.0),
            &format!("Pace: {}", session.pace.display_name()),
            true,
            mouse,
        ) {
            actions.push(SettingsAction::TogglePace);
        }

        if menu_button(
            Rect::new(LOGICAL_WIDTH - 220.0, LOGICAL_HEIGHT - 58.0, 150.0, 40.0),
            "Back [Esc]",
            true,
            mouse,
        ) {
            actions.push(SettingsAction::Back);
        }
        actions
    }
}
