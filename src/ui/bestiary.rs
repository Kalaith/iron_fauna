//! The Bestiary — a viewing surface for the collection pillar
//! (`game_design.md` §6). Every species the world holds, caught ones revealed
//! in full with their chassis stats, the rest left as silhouettes to chase.

use crate::data::species::SpeciesDef;
use crate::data::GameData;
use crate::state::GameSession;
use crate::ui::creature_art;
use crate::ui::{element_color, menu_button, LOGICAL_HEIGHT, LOGICAL_WIDTH};
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::{draw_ui_text_ex, RectExt, VirtualUi};

pub struct BestiaryScreen;

const COLS: usize = 6;
const CELL_W: f32 = 198.0;
const CELL_H: f32 = 80.0;
const GRID_X: f32 = 28.0;
const GRID_Y: f32 = 108.0;

impl BestiaryScreen {
    /// Returns true when the player dismisses the screen.
    pub fn draw(&self, data: &GameData, session: &GameSession, ui: &VirtualUi) -> bool {
        let mut close = is_key_pressed(KeyCode::Escape);
        let mouse = ui.mouse_position();
        clear_background(Color::new(0.05, 0.055, 0.07, 1.0));

        // Species in a stable display order: tier, then id.
        let mut species: Vec<&SpeciesDef> = data.species.iter().map(|(_, s)| s).collect();
        species.sort_by(|a, b| a.tier.cmp(&b.tier).then(a.id.cmp(&b.id)));

        let caught = species
            .iter()
            .filter(|s| session.profile.roster.owns_species(&s.id))
            .count();

        draw_ui_text_ex(
            "BESTIARY",
            GRID_X,
            72.0,
            TextStyle::new(36.0, Color::new(0.88, 0.86, 0.80, 1.0)).params(),
        );
        draw_ui_text_ex(
            &format!("{} of {} cores collected", caught, species.len()),
            GRID_X + 260.0,
            68.0,
            TextStyle::new(18.0, dark::TEXT_DIM).params(),
        );

        let mut hovered: Option<&SpeciesDef> = None;
        for (i, sp) in species.iter().enumerate() {
            let col = i % COLS;
            let row = i / COLS;
            let rect = Rect::new(
                GRID_X + col as f32 * (CELL_W + 6.0),
                GRID_Y + row as f32 * (CELL_H + 6.0),
                CELL_W,
                CELL_H,
            );
            let owned = session.profile.roster.owns_species(&sp.id);
            let is_hovered = rect.contains_point(mouse);
            if is_hovered {
                hovered = Some(sp);
            }
            self.draw_cell(data, session, rect, sp, owned, is_hovered);
        }

        // Detail bar for the hovered species.
        self.draw_detail(data, session, hovered, mouse);

        if menu_button(
            Rect::new(LOGICAL_WIDTH - 190.0, 44.0, 150.0, 38.0),
            "Back [Esc]",
            true,
            mouse,
        ) {
            close = true;
        }
        close
    }

    fn draw_cell(
        &self,
        data: &GameData,
        session: &GameSession,
        rect: Rect,
        sp: &SpeciesDef,
        owned: bool,
        hovered: bool,
    ) {
        let fill = if hovered {
            Color::new(0.14, 0.16, 0.20, 1.0)
        } else {
            Color::new(0.09, 0.10, 0.13, 1.0)
        };
        let accent = if owned {
            element_color(sp.element)
        } else {
            Color::new(0.28, 0.30, 0.34, 1.0)
        };
        draw_surface(
            rect,
            &SurfaceStyle::new(fill)
                .with_border(1.0, Color::new(0.28, 0.32, 0.40, 0.6))
                .with_left_accent(4.0, accent),
        );

        // A procedurally-generated portrait of the chassis; uncaught species
        // show only their silhouette, a shape to chase down in the wild.
        let cx = rect.x + 32.0;
        let cy = rect.y + rect.h * 0.5;
        if owned {
            creature_art::draw_war_body(cx, cy, 19.0, sp, &[]);
        } else {
            creature_art::draw_silhouette(cx, cy, 19.0, sp);
        }

        let name = if owned {
            sp.name.as_str()
        } else {
            "— unknown —"
        };
        draw_ui_text_ex(
            name,
            rect.x + 56.0,
            rect.y + 26.0,
            TextStyle::new(
                16.0,
                if owned {
                    dark::TEXT_BRIGHT
                } else {
                    dark::TEXT_DIM
                },
            )
            .params(),
        );
        if owned {
            draw_ui_text_ex(
                &format!(
                    "{} {} · {}",
                    sp.size.display_name(),
                    sp.archetype.display_name(),
                    sp.element.display_name()
                ),
                rect.x + 56.0,
                rect.y + 46.0,
                TextStyle::new(12.0, dark::TEXT_DIM).params(),
            );
            let count = session.profile.roster.species_count(&sp.id);
            if count > 1 {
                draw_text_right(
                    &format!("×{}", count),
                    rect.right() - 10.0,
                    rect.y + 24.0,
                    TextStyle::new(13.0, dark::TEXT),
                );
            }
        }
        // Tier pips, always shown (a hint at where it lives).
        for t in 0..sp.tier {
            draw_rectangle(
                rect.x + 56.0 + t as f32 * 8.0,
                rect.bottom() - 14.0,
                5.0,
                5.0,
                if owned {
                    accent
                } else {
                    Color::new(0.3, 0.32, 0.36, 1.0)
                },
            );
        }
        let _ = data;
    }

    fn draw_detail(
        &self,
        data: &GameData,
        session: &GameSession,
        hovered: Option<&SpeciesDef>,
        _mouse: Vec2,
    ) {
        let bar = Rect::new(28.0, LOGICAL_HEIGHT - 92.0, LOGICAL_WIDTH - 56.0, 78.0);
        draw_surface(
            bar,
            &SurfaceStyle::new(Color::new(0.07, 0.075, 0.10, 0.97))
                .with_border(1.0, Color::new(0.35, 0.40, 0.50, 0.6)),
        );
        let text_w = bar.w - 320.0;

        // Always-on before/after: the hovered owned chassis, or the first one
        // collected. Bare chassis on the left, the same chassis grafted on the
        // right — the whole point of the game in one glance.
        let featured = hovered
            .filter(|s| session.profile.roster.owns_species(&s.id))
            .or_else(|| self.first_owned(data, session));
        if let Some(sp) = featured {
            self.draw_before_after(sp, bar);
        }

        let Some(sp) = hovered else {
            draw_text_block(
                "Hover a core to inspect its chassis. Catch them in the wild or free them from the factories.",
                bar.x + 18.0,
                bar.y + 30.0,
                text_w,
                44.0,
                15.0,
                5.0,
                dark::TEXT_DIM,
            );
            return;
        };
        let owned = session.profile.roster.owns_species(&sp.id);
        if !owned {
            draw_ui_text_ex(
                "An unknown core. Somewhere out there it is small and afraid and waiting.",
                bar.x + 18.0,
                bar.y + 44.0,
                TextStyle::new(15.0, dark::TEXT_DIM).params(),
            );
            return;
        }
        let d = sp.derived(&data.balance);
        draw_ui_text_ex(
            &format!(
                "{}  —  Pow {}  Spd {}  ·  {} mounts  ·  Vigor {:.0}  ·  {} temperament",
                sp.name,
                sp.power,
                sp.speed,
                sp.mount_count(),
                d.vigor_max,
                sp.temperament.display_name(),
            ),
            bar.x + 18.0,
            bar.y + 26.0,
            TextStyle::new(15.0, element_color(sp.element)).params(),
        );
        draw_text_block(
            &sp.description,
            bar.x + 18.0,
            bar.y + 38.0,
            text_w,
            36.0,
            14.0,
            4.0,
            dark::TEXT,
        );
    }

    /// First collected species in the grid's display order (tier, then id).
    fn first_owned<'a>(&self, data: &'a GameData, session: &GameSession) -> Option<&'a SpeciesDef> {
        let mut owned: Vec<&SpeciesDef> = data
            .species
            .iter()
            .map(|(_, s)| s)
            .filter(|s| session.profile.roster.owns_species(&s.id))
            .collect();
        owned.sort_by(|a, b| a.tier.cmp(&b.tier).then(a.id.cmp(&b.id)));
        owned.into_iter().next()
    }

    /// Bare chassis → grafted, side by side, with an arrow between.
    fn draw_before_after(&self, sp: &SpeciesDef, bar: Rect) {
        let cy = bar.y + bar.h * 0.5 - 4.0;
        let u = 24.0;
        let bare_cx = bar.right() - 210.0;
        let graft_cx = bar.right() - 74.0;

        creature_art::draw_war_body(bare_cx, cy, u, sp, &[]);
        creature_art::draw_war_body(graft_cx, cy, u, sp, &creature_art::sample_grafts(sp));

        // Arrow drawn from primitives — the display font lacks a glyph for it.
        let mx = (bare_cx + graft_cx) * 0.5;
        let arrow = dark::TEXT_DIM;
        draw_line(mx - 13.0, cy, mx + 6.0, cy, 3.0, arrow);
        draw_triangle(
            vec2(mx + 13.0, cy),
            vec2(mx + 4.0, cy - 6.0),
            vec2(mx + 4.0, cy + 6.0),
            arrow,
        );
        for (cx, label) in [(bare_cx, "chassis"), (graft_cx, "grafted")] {
            draw_text_centered_in_box_ex(
                label,
                cx - 50.0,
                bar.bottom() - 15.0,
                100.0,
                12.0,
                TextStyle::new(11.0, dark::TEXT_DIM),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::data::GameData;
    use crate::state::GameSession;

    #[test]
    fn collection_tracks_owned_species() {
        let data = GameData::load().unwrap();
        let mut session = GameSession::new_game(&data);
        // Starter Volpi is owned; a never-caught species is not.
        assert!(session.profile.roster.owns_species("volpi"));
        assert!(!session.profile.roster.owns_species("tembolo"));
        session.profile.spawn_creature(
            &data,
            "tembolo",
            crate::model::creature::CreatureOrigin::Wild,
        );
        assert!(session.profile.roster.owns_species("tembolo"));
        assert_eq!(session.profile.roster.species_count("volpi"), 1);
    }
}
