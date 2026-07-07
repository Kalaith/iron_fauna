//! Overworld tile rendering: painting one map tile from the PostApocalypse
//! sprite set (ground fills from mood-variant tilesets, plus tree/rock/door
//! objects), with a procedural fallback when textures are unavailable.

use super::TILE;
use crate::data::world::{MapKind, TileKind};
use crate::model::worldstate::RegionMood;
use macroquad::prelude::*;
use macroquad_toolkit::assets::AssetManager;

/// Source cell size in the background tilesets (16×16 pixel art).
const CELL: f32 = 16.0;

/// Which mood-variant tileset paints the ground.
fn terrain_key(mood: RegionMood) -> &'static str {
    match mood {
        RegionMood::Reviving | RegionMood::Claimed => "terrain_green",
        RegionMood::Threatened => "terrain_dark",
        RegionMood::Dead | RegionMood::Relapsed => "terrain_bleak",
    }
}

/// Matching suffix for the mood-variant object sprites (trees, bushes, doors).
fn obj_suffix(mood: RegionMood) -> &'static str {
    match mood {
        RegionMood::Reviving | RegionMood::Claimed => "green",
        RegionMood::Threatened => "dark_green",
        RegionMood::Dead | RegionMood::Relapsed => "bleak_yellow",
    }
}

fn tile_hash(tx: i32, ty: i32) -> f32 {
    ((tx.wrapping_mul(73_856_093) ^ ty.wrapping_mul(19_349_663)) as u32 >> 8) as f32 / 16_777_216.0
}

/// Blit one 16×16 cell of a tileset scaled to fill a map tile.
fn blit_cell(tex: &Texture2D, c: i32, r: i32, px: f32, py: f32, tint: Color) {
    draw_texture_ex(
        tex,
        px,
        py,
        tint,
        DrawTextureParams {
            source: Some(Rect::new(c as f32 * CELL, r as f32 * CELL, CELL, CELL)),
            dest_size: Some(vec2(TILE, TILE)),
            ..Default::default()
        },
    );
}

/// Blit a free-standing object sprite, centred and bottom-anchored on the tile
/// so tall props (trees, doorways) overhang upward onto the row above.
fn blit_object(tex: &Texture2D, px: f32, py: f32, target_w: f32) {
    let (tw, th) = (tex.width(), tex.height());
    let dw = target_w;
    let dh = th * (dw / tw);
    draw_texture_ex(
        tex,
        px + (TILE - dw) * 0.5,
        py + TILE - dh,
        WHITE,
        DrawTextureParams {
            dest_size: Some(vec2(dw, dh)),
            ..Default::default()
        },
    );
}

/// Paint one overworld tile. Factory floors and any missing textures fall back
/// to the procedural renderer so the game always draws something.
pub(super) fn draw_tile(
    assets: &AssetManager,
    kind: TileKind,
    pos: Vec2,
    cell: (i32, i32),
    map_kind: MapKind,
    mood: RegionMood,
) {
    let (px, py) = (pos.x, pos.y);
    let (tx, ty) = cell;
    let terrain = assets.get_texture(terrain_key(mood));
    if map_kind == MapKind::Factory || terrain.is_none() {
        draw_tile_procedural(kind, px, py, tx, ty, map_kind, mood);
        return;
    }
    let terrain = terrain.unwrap();
    let h = tile_hash(tx, ty);
    let obj = |name: &str| assets.get_texture(&format!("{}_{}", name, obj_suffix(mood)));

    match kind {
        TileKind::Ground => blit_cell(terrain, 5, 0, px, py, WHITE),
        TileKind::Path => blit_cell(terrain, 1, 10, px, py, WHITE),
        TileKind::Grass => {
            blit_cell(terrain, 5, 0, px, py, WHITE);
            // Tall grass reads as scrub — the odd bush breaks up the field.
            if h > 0.62 {
                if let Some(b) = obj("bush") {
                    blit_object(b, px, py, TILE * 0.7);
                }
            }
        }
        TileKind::Tree => {
            blit_cell(terrain, 5, 0, px, py, WHITE);
            let key = if h > 0.5 { "tree" } else { "tree2" };
            if let Some(t) = obj(key) {
                blit_object(t, px, py, TILE * 0.92);
            } else {
                draw_tile_procedural(kind, px, py, tx, ty, map_kind, mood);
            }
        }
        TileKind::Rock => blit_cell(terrain, 13, 15, px, py, WHITE),
        TileKind::SettlementDoor => {
            blit_cell(terrain, 1, 10, px, py, WHITE);
            if let Some(e) = obj("entrance") {
                blit_object(e, px, py, TILE);
            }
        }
        TileKind::GestariumDoor => {
            blit_cell(terrain, 1, 10, px, py, WHITE);
            if let Some(d) = assets.get_texture("door_metal") {
                blit_object(d, px, py, TILE * 0.8);
            }
        }
        // Water and the factory-only kinds keep their hand-drawn look.
        _ => draw_tile_procedural(kind, px, py, tx, ty, map_kind, mood),
    }
}

/// Region moods tint the land (`game_design.md` §9.2): purged ground grays
/// out, reseeded ground blooms.
fn mood_tint(mood: RegionMood) -> (f32, f32, f32) {
    match mood {
        RegionMood::Threatened => (1.0, 1.0, 1.0),
        RegionMood::Dead => (1.0, 0.72, 0.65),
        RegionMood::Reviving => (0.9, 1.25, 0.9),
        RegionMood::Claimed => (1.0, 0.95, 1.15),
        RegionMood::Relapsed => (1.25, 0.8, 0.75),
    }
}

fn tinted(c: Color, t: (f32, f32, f32)) -> Color {
    Color::new(
        (c.r * t.0).min(1.0),
        (c.g * t.1).min(1.0),
        (c.b * t.2).min(1.0),
        c.a,
    )
}

fn draw_tile_procedural(
    kind: TileKind,
    px: f32,
    py: f32,
    tx: i32,
    ty: i32,
    map_kind: MapKind,
    mood: RegionMood,
) {
    // Deterministic per-tile jitter for texture without an RNG.
    let h = ((tx.wrapping_mul(73_856_093) ^ ty.wrapping_mul(19_349_663)) as u32 >> 8) as f32
        / 16_777_216.0;
    let t = mood_tint(mood);
    match kind {
        TileKind::Ground => {
            draw_rectangle(
                px,
                py,
                TILE,
                TILE,
                tinted(Color::new(0.16, 0.19, 0.13, 1.0), t),
            );
            if h > 0.8 {
                draw_circle(
                    px + TILE * 0.3,
                    py + TILE * 0.6,
                    2.0,
                    tinted(Color::new(0.20, 0.24, 0.16, 1.0), t),
                );
            }
        }
        TileKind::Path => {
            draw_rectangle(
                px,
                py,
                TILE,
                TILE,
                tinted(Color::new(0.26, 0.22, 0.16, 1.0), t),
            );
        }
        TileKind::Grass => {
            draw_rectangle(
                px,
                py,
                TILE,
                TILE,
                tinted(Color::new(0.11, 0.23, 0.12, 1.0), t),
            );
            let sway = h * 6.0;
            draw_line(
                px + 8.0 + sway,
                py + TILE - 6.0,
                px + 10.0 + sway,
                py + 10.0,
                2.0,
                tinted(Color::new(0.16, 0.33, 0.17, 1.0), t),
            );
            draw_line(
                px + 24.0 + sway,
                py + TILE - 6.0,
                px + 26.0 + sway,
                py + 14.0,
                2.0,
                tinted(Color::new(0.14, 0.30, 0.15, 1.0), t),
            );
        }
        TileKind::Tree => {
            if map_kind == MapKind::Factory {
                // Factory walls: riveted plating, not trees.
                draw_rectangle(px, py, TILE, TILE, Color::new(0.13, 0.13, 0.16, 1.0));
                draw_rectangle_lines(px, py, TILE, TILE, 2.0, Color::new(0.20, 0.20, 0.24, 1.0));
            } else {
                draw_rectangle(
                    px,
                    py,
                    TILE,
                    TILE,
                    tinted(Color::new(0.09, 0.13, 0.09, 1.0), t),
                );
                draw_circle(
                    px + TILE * 0.5,
                    py + TILE * 0.4,
                    TILE * 0.38,
                    tinted(Color::new(0.07, 0.17, 0.10, 1.0), t),
                );
            }
        }
        TileKind::DeckPlate => {
            draw_rectangle(px, py, TILE, TILE, Color::new(0.11, 0.115, 0.13, 1.0));
            if h > 0.7 {
                draw_line(
                    px + 4.0,
                    py + TILE - 4.0,
                    px + TILE - 4.0,
                    py + TILE - 4.0,
                    1.0,
                    Color::new(0.16, 0.165, 0.19, 1.0),
                );
            }
        }
        TileKind::VatSpill => {
            draw_rectangle(px, py, TILE, TILE, Color::new(0.10, 0.14, 0.11, 1.0));
            draw_circle(
                px + TILE * 0.5,
                py + TILE * 0.55,
                TILE * 0.32,
                Color::new(0.18, 0.34, 0.20, 0.9),
            );
            draw_circle(
                px + TILE * 0.4,
                py + TILE * 0.45,
                TILE * 0.12,
                Color::new(0.30, 0.55, 0.30, 0.8),
            );
        }
        TileKind::Vat => {
            draw_rectangle(px, py, TILE, TILE, Color::new(0.10, 0.11, 0.13, 1.0));
            draw_rectangle(
                px + 6.0,
                py + 3.0,
                TILE - 12.0,
                TILE - 6.0,
                Color::new(0.16, 0.22, 0.24, 1.0),
            );
            // The small sleeping core inside — the horror is that it's cute.
            draw_circle(
                px + TILE * 0.5,
                py + TILE * 0.5,
                6.0,
                Color::new(0.85, 0.65, 0.70, 0.9),
            );
        }
        TileKind::Heart => {
            draw_rectangle(px, py, TILE, TILE, Color::new(0.12, 0.08, 0.10, 1.0));
            draw_circle(
                px + TILE * 0.5,
                py + TILE * 0.5,
                TILE * 0.42,
                Color::new(0.45, 0.16, 0.22, 1.0),
            );
            draw_circle(
                px + TILE * 0.5,
                py + TILE * 0.5,
                TILE * 0.22,
                Color::new(0.75, 0.30, 0.38, 1.0),
            );
        }
        TileKind::Water => {
            draw_rectangle(px, py, TILE, TILE, Color::new(0.10, 0.18, 0.30, 1.0));
            draw_line(
                px + 6.0,
                py + TILE * 0.5 + h * 8.0,
                px + TILE - 6.0,
                py + TILE * 0.5 + h * 8.0,
                1.5,
                Color::new(0.16, 0.26, 0.40, 1.0),
            );
        }
        TileKind::Rock => {
            draw_rectangle(px, py, TILE, TILE, Color::new(0.16, 0.16, 0.17, 1.0));
            draw_circle(
                px + TILE * 0.5,
                py + TILE * 0.55,
                TILE * 0.3,
                Color::new(0.24, 0.24, 0.26, 1.0),
            );
        }
        TileKind::SettlementDoor => {
            draw_rectangle(px, py, TILE, TILE, Color::new(0.30, 0.24, 0.15, 1.0));
            draw_rectangle(
                px + 10.0,
                py + 6.0,
                TILE - 20.0,
                TILE - 12.0,
                Color::new(0.45, 0.35, 0.20, 1.0),
            );
        }
        TileKind::GestariumDoor => {
            draw_rectangle(px, py, TILE, TILE, Color::new(0.14, 0.10, 0.12, 1.0));
            draw_rectangle(
                px + 6.0,
                py + 4.0,
                TILE - 12.0,
                TILE - 8.0,
                Color::new(0.28, 0.14, 0.18, 1.0),
            );
            draw_circle(
                px + TILE * 0.5,
                py + TILE * 0.5,
                4.0,
                Color::new(0.65, 0.30, 0.30, 1.0),
            );
        }
    }
}
