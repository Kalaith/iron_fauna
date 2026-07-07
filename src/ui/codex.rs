//! The Codex — a Tab-opened overview the rider carries everywhere but battle:
//! status, corelings, party order, quests, and the journal. A pure view layer
//! that reads state and returns intents; `game.rs` applies them.

mod tabs;

use crate::data::GameData;
use crate::state::GameSession;
use crate::ui::{menu_button, LOGICAL_HEIGHT, LOGICAL_WIDTH};
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::{draw_ui_text_ex, RectExt, VirtualUi};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodexTab {
    Status,
    Corelings,
    Party,
    Quests,
    Journal,
}

impl CodexTab {
    pub const ALL: [CodexTab; 5] = [
        CodexTab::Status,
        CodexTab::Corelings,
        CodexTab::Party,
        CodexTab::Quests,
        CodexTab::Journal,
    ];

    fn label(self) -> &'static str {
        match self {
            CodexTab::Status => "Status",
            CodexTab::Corelings => "Corelings",
            CodexTab::Party => "Party",
            CodexTab::Quests => "Quests",
            CodexTab::Journal => "Journal",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodexAction {
    Close,
    Show(CodexTab),
    Select(u64),
    MoveUp(u64),
    MoveDown(u64),
    ToParty(u64),
    ToStorage(u64),
}

pub struct CodexScreen {
    pub tab: CodexTab,
    /// Currently inspected coreling (Corelings tab).
    pub selected: Option<u64>,
    /// Set when opened from a settlement, so closing returns there.
    pub return_settlement: Option<String>,
}

impl CodexScreen {
    pub fn new(session: &GameSession, return_settlement: Option<String>) -> Self {
        Self {
            tab: CodexTab::Status,
            selected: session.profile.roster.party.first().copied(),
            return_settlement,
        }
    }

    pub fn draw(&self, data: &GameData, session: &GameSession, ui: &VirtualUi) -> Vec<CodexAction> {
        let mut actions = Vec::new();
        let mouse = ui.mouse_position();
        clear_background(Color::new(0.05, 0.055, 0.07, 1.0));

        if is_key_pressed(KeyCode::Escape) || is_key_pressed(KeyCode::Tab) {
            actions.push(CodexAction::Close);
        }
        // Number keys jump straight to a tab.
        for (i, key) in [
            KeyCode::Key1,
            KeyCode::Key2,
            KeyCode::Key3,
            KeyCode::Key4,
            KeyCode::Key5,
        ]
        .into_iter()
        .enumerate()
        {
            if is_key_pressed(key) {
                actions.push(CodexAction::Show(CodexTab::ALL[i]));
            }
        }

        self.draw_tab_bar(mouse, &mut actions);

        // A dark surface for the body — the tab text is light-on-dark, so the
        // cream skin panel (great behind buttons) would wash it out here.
        let body = Rect::new(24.0, 132.0, LOGICAL_WIDTH - 48.0, LOGICAL_HEIGHT - 156.0);
        draw_surface(
            body,
            &SurfaceStyle::new(Color::new(0.08, 0.085, 0.11, 0.98))
                .with_border(1.0, Color::new(0.35, 0.4, 0.5, 0.6)),
        );
        let content = body.inset(22.0);
        match self.tab {
            CodexTab::Status => tabs::status(data, session, content),
            CodexTab::Corelings => {
                tabs::corelings(self, data, session, content, mouse, &mut actions)
            }
            CodexTab::Party => tabs::party(data, session, content, mouse, &mut actions),
            CodexTab::Quests => tabs::quests(data, session, content),
            CodexTab::Journal => tabs::journal(data, session, content),
        }
        actions
    }

    fn draw_tab_bar(&self, mouse: Vec2, actions: &mut Vec<CodexAction>) {
        draw_ui_text_ex(
            "CODEX",
            28.0,
            48.0,
            TextStyle::new(30.0, Color::new(0.88, 0.86, 0.80, 1.0)).params(),
        );
        draw_ui_text_ex(
            "[Tab] / [Esc] close   ·   [1-5] jump",
            LOGICAL_WIDTH - 320.0,
            44.0,
            TextStyle::new(15.0, dark::TEXT_DIM).params(),
        );
        let bw = 150.0;
        for (i, tab) in CodexTab::ALL.into_iter().enumerate() {
            let rect = Rect::new(28.0 + i as f32 * (bw + 8.0), 76.0, bw, 40.0);
            if tab == self.tab {
                // Highlight the active tab with an accent bar under it.
                draw_rectangle(
                    rect.x,
                    rect.bottom() + 2.0,
                    rect.w,
                    4.0,
                    Color::new(0.60, 0.75, 0.95, 1.0),
                );
            }
            if menu_button(rect, tab.label(), true, mouse) {
                actions.push(CodexAction::Show(tab));
            }
        }
    }
}
