//! The World Ledger — a running account of every region's fate, and the
//! closing reflection once all six Gestaria are judged (`game_design.md` §9.2:
//! the world's final state is the sum of the player's verdicts).

use crate::data::GameData;
use crate::model::worldstate::RegionMood;
use crate::state::GameSession;
use crate::ui::{menu_button, LOGICAL_HEIGHT, LOGICAL_WIDTH};
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::{draw_ui_text_ex, VirtualUi};

pub struct LedgerScreen;

pub fn mood_color(mood: RegionMood) -> Color {
    match mood {
        RegionMood::Threatened => Color::new(0.70, 0.62, 0.45, 1.0),
        RegionMood::Dead => Color::new(0.55, 0.50, 0.52, 1.0),
        RegionMood::Reviving => Color::new(0.45, 0.75, 0.45, 1.0),
        RegionMood::Claimed => Color::new(0.62, 0.52, 0.80, 1.0),
        RegionMood::Relapsed => Color::new(0.85, 0.40, 0.35, 1.0),
    }
}

impl LedgerScreen {
    /// Returns true when the player dismisses the screen.
    pub fn draw(&self, data: &GameData, session: &GameSession, ui: &VirtualUi) -> bool {
        let mut close = is_key_pressed(KeyCode::Escape);
        let mouse = ui.mouse_position();

        clear_background(Color::new(0.04, 0.045, 0.06, 1.0));
        let all_judged = session.world_state.all_judged(data);
        let passed = session.world_state.verdicts_passed(data);
        let total = data.factories.len();

        let title = if all_judged {
            "THE WORLD YOU LEAVE"
        } else {
            "THE WORLD LEDGER"
        };
        draw_ui_text_ex(
            title,
            72.0,
            88.0,
            TextStyle::new(38.0, Color::new(0.88, 0.86, 0.80, 1.0)).params(),
        );
        draw_ui_text_ex(
            &format!("{} of {} Gestaria judged", passed, total),
            74.0,
            120.0,
            TextStyle::new(17.0, dark::TEXT_DIM).params(),
        );

        // One row per region, in the world's canonical order.
        let mut y = 150.0;
        for region in &data.world.regions {
            let mood = session.world_state.region_mood(data, &region.id);
            let state = session.world_state.factory(&region.gestarium_id);
            let row = Rect::new(72.0, y, LOGICAL_WIDTH - 144.0, 62.0);
            draw_surface(
                row,
                &SurfaceStyle::new(Color::new(0.08, 0.085, 0.11, 0.95))
                    .with_border(1.0, Color::new(0.30, 0.34, 0.42, 0.6))
                    .with_left_accent(5.0, mood_color(mood)),
            );
            draw_ui_text_ex(
                &region.name,
                row.x + 20.0,
                row.y + 28.0,
                TextStyle::new(21.0, dark::TEXT_BRIGHT).params(),
            );
            let fate = match state.verdict {
                Some(v) => {
                    if state.relapsed {
                        format!("{} — and relapsed", v.display_name())
                    } else if v == crate::model::worldstate::Verdict::Reseed && state.invested {
                        format!("{} — watched", v.display_name())
                    } else {
                        v.display_name().to_owned()
                    }
                }
                None if state.heart_defeated => "silenced, unjudged".to_owned(),
                None => "still threatened".to_owned(),
            };
            draw_ui_text_ex(
                &fate,
                row.x + 20.0,
                row.y + 52.0,
                TextStyle::new(15.0, mood_color(mood)).params(),
            );
            draw_text_right(
                &format!("the region is {}", mood.display_name()),
                row.right() - 20.0,
                row.y + 36.0,
                TextStyle::new(15.0, dark::TEXT_DIM),
            );
            y += 70.0;
        }

        // Closing reflection once everything is judged.
        if all_judged {
            let (purge, reseed, bind) = session.world_state.verdict_tally(data);
            let reflection = closing_line(purge, reseed, bind);
            let box_rect = Rect::new(72.0, y + 6.0, LOGICAL_WIDTH - 144.0, 96.0);
            draw_surface(
                box_rect,
                &SurfaceStyle::new(Color::new(0.06, 0.06, 0.09, 0.97))
                    .with_border(1.0, Color::new(0.55, 0.50, 0.40, 0.7)),
            );
            draw_text_block(
                reflection,
                box_rect.x + 20.0,
                box_rect.y + 16.0,
                box_rect.w - 40.0,
                box_rect.h - 24.0,
                18.0,
                6.0,
                Color::new(0.85, 0.82, 0.75, 1.0),
            );
        }

        if menu_button(
            Rect::new(LOGICAL_WIDTH - 220.0, LOGICAL_HEIGHT - 58.0, 150.0, 40.0),
            "Back [Esc]",
            true,
            mouse,
        ) {
            close = true;
        }
        close
    }
}

/// The epilogue's single closing sentence, chosen by the shape of the ledger.
fn closing_line(purge: usize, reseed: usize, bind: usize) -> &'static str {
    if bind >= 4 {
        "You hold more of the old machines than any Progenitor ever did. You told yourself, each time, that you were different. Somewhere a child is being taught your name the way you were taught theirs."
    } else if purge >= 4 {
        "You made the world safe the way a graveyard is safe. The people you saved will grow old in the quiet, and never quite forgive the silence you gave them. Neither will you."
    } else if reseed >= 4 {
        "You gave the world back its green, and a fistful of loaded guns to guard it. It is alive again, and always one careless season from becoming what it was. You will spend the rest of your life watching. That is the cost of hope, and you paid it gladly."
    } else {
        "Some regions you buried. Some you brought back. Some you kept. No single word fits the map you leave behind — only your name, written across all of it, in the only handwriting the world will remember: the choices no one else was left to make."
    }
}

#[cfg(test)]
mod tests {
    use crate::data::GameData;
    use crate::model::worldstate::Verdict;
    use crate::state::GameSession;

    #[test]
    fn all_judged_only_when_every_factory_decided() {
        let data = GameData::load().unwrap();
        let mut session = GameSession::new_game(&data);
        assert!(!session.world_state.all_judged(&data));
        for (id, _) in data.factories.iter() {
            session.world_state.factory_mut(id).verdict = Some(Verdict::Reseed);
        }
        assert!(session.world_state.all_judged(&data));
        let (_, reseed, _) = session.world_state.verdict_tally(&data);
        assert_eq!(reseed, data.factories.len());
    }
}
