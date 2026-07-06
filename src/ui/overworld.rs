//! Overworld traversal: connected tile maps, grid movement, wild encounters,
//! NPC dialogue, and doors into settlements and Gestaria (`game_design.md` §8).

use crate::combat::unit::UnitSpec;
use crate::combat::{Side, Stance};
use crate::data::world::{MapDef, NpcDef, TileKind};
use crate::data::GameData;
use crate::state::GameSession;
use crate::ui::{LOGICAL_HEIGHT, LOGICAL_WIDTH};
use crate::util::Rng;
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::draw_ui_text_ex;

const TILE: f32 = 40.0;

pub enum OverworldResult {
    Continue,
    /// Stepped through a settlement door — open the bench.
    OpenSettlement,
    /// Walked into a wild pack.
    StartEncounter(Vec<UnitSpec>),
    BackToMenu,
}

struct DialogueBox {
    name: String,
    lines: Vec<String>,
    index: usize,
}

pub struct OverworldScreen {
    move_timer: f32,
    facing: (i32, i32),
    dialogue: Option<DialogueBox>,
    rng: Rng,
}

impl OverworldScreen {
    pub fn new(session: &GameSession) -> Self {
        Self {
            move_timer: 0.0,
            facing: (0, 1),
            dialogue: None,
            rng: Rng::new(
                0x9E37_79B9_7F4A_7C15
                    ^ (session.steps.wrapping_mul(0x2545_F491_4F6C_DD1D))
                    ^ session.battles_fought as u64,
            ),
        }
    }

    pub fn update(
        &mut self,
        data: &GameData,
        session: &mut GameSession,
        dt: f32,
    ) -> OverworldResult {
        if is_key_pressed(KeyCode::Escape) && self.dialogue.is_none() {
            return OverworldResult::BackToMenu;
        }

        let Some(map) = data.world.map(&session.location.map_id) else {
            return OverworldResult::BackToMenu;
        };

        // Dialogue swallows input until dismissed.
        if let Some(dialog) = &mut self.dialogue {
            if is_key_pressed(KeyCode::Space)
                || is_key_pressed(KeyCode::Enter)
                || is_key_pressed(KeyCode::Escape)
            {
                dialog.index += 1;
                if dialog.index >= dialog.lines.len() {
                    self.dialogue = None;
                }
            }
            return OverworldResult::Continue;
        }

        // Interact with whatever we're facing.
        if is_key_pressed(KeyCode::Space) || is_key_pressed(KeyCode::Enter) {
            let fx = session.location.x + self.facing.0;
            let fy = session.location.y + self.facing.1;
            if let Some(npc) = map.npc_at(fx, fy) {
                self.dialogue = Some(npc_dialogue(npc));
                return OverworldResult::Continue;
            }
            if map.tile(fx, fy) == TileKind::GestariumDoor {
                self.dialogue = Some(DialogueBox {
                    name: "Sealed Doors".to_owned(),
                    lines: vec![
                        "Warm air breathes through the seam. Deep in the hum of the earth, something is still growing.".to_owned(),
                        "The doors do not answer. Not yet.".to_owned(),
                    ],
                    index: 0,
                });
                return OverworldResult::Continue;
            }
        }

        self.move_timer -= dt;
        let dir = held_direction();
        if let Some(dir) = dir {
            self.facing = dir;
            if self.move_timer <= 0.0 {
                let nx = session.location.x + dir.0;
                let ny = session.location.y + dir.1;
                if map.walkable(nx, ny) {
                    session.location.x = nx;
                    session.location.y = ny;
                    session.steps += 1;
                    self.move_timer = step_time(data, session);

                    if let Some(warp) = map.warp_at(nx, ny) {
                        session.location.map_id = warp.to_map.clone();
                        session.location.x = warp.to_x;
                        session.location.y = warp.to_y;
                        return OverworldResult::Continue;
                    }
                    match map.tile(nx, ny) {
                        TileKind::SettlementDoor => return OverworldResult::OpenSettlement,
                        TileKind::Grass if self.rng.chance(map.encounter_rate) => {
                            if let Some(pack) = roll_encounter(map, data, &mut self.rng) {
                                return OverworldResult::StartEncounter(pack);
                            }
                        }
                        _ => {}
                    }
                } else {
                    // Bumping still turns the step timer over slightly.
                    self.move_timer = 0.08;
                }
            }
        }
        OverworldResult::Continue
    }

    pub fn draw(&self, data: &GameData, session: &GameSession) {
        let Some(map) = data.world.map(&session.location.map_id) else {
            return;
        };

        // Camera: center on player, clamped to the map bounds.
        let view_w = (LOGICAL_WIDTH / TILE).ceil() as i32;
        let view_h = (LOGICAL_HEIGHT / TILE).ceil() as i32;
        let cam_x = (session.location.x - view_w / 2).clamp(0, (map.width() - view_w).max(0));
        let cam_y = (session.location.y - view_h / 2).clamp(0, (map.height() - view_h).max(0));

        clear_background(Color::new(0.05, 0.06, 0.05, 1.0));
        for ty in cam_y..(cam_y + view_h + 1).min(map.height()) {
            for tx in cam_x..(cam_x + view_w + 1).min(map.width()) {
                let px = (tx - cam_x) as f32 * TILE;
                let py = (ty - cam_y) as f32 * TILE;
                draw_tile(map.tile(tx, ty), px, py, tx, ty);
            }
        }

        for npc in &map.npcs {
            if npc.x < cam_x || npc.y < cam_y {
                continue;
            }
            let px = (npc.x - cam_x) as f32 * TILE + TILE * 0.5;
            let py = (npc.y - cam_y) as f32 * TILE + TILE * 0.5;
            draw_circle(px, py - 6.0, 9.0, Color::new(0.80, 0.70, 0.55, 1.0));
            draw_rectangle(px - 7.0, py, 14.0, 14.0, Color::new(0.45, 0.40, 0.55, 1.0));
        }

        // Player.
        let px = (session.location.x - cam_x) as f32 * TILE + TILE * 0.5;
        let py = (session.location.y - cam_y) as f32 * TILE + TILE * 0.5;
        draw_circle(px, py - 7.0, 9.0, Color::new(0.92, 0.85, 0.72, 1.0));
        draw_rectangle(
            px - 7.0,
            py - 1.0,
            14.0,
            15.0,
            Color::new(0.55, 0.35, 0.28, 1.0),
        );
        // Facing tick.
        draw_circle(
            px + self.facing.0 as f32 * 10.0,
            py - 7.0 + self.facing.1 as f32 * 6.0,
            2.5,
            Color::new(0.2, 0.15, 0.1, 1.0),
        );

        self.draw_hud(data, map);
        if let Some(dialog) = &self.dialogue {
            draw_dialogue(dialog);
        }
    }

    fn draw_hud(&self, data: &GameData, map: &MapDef) {
        let region = data
            .world
            .region(&map.region)
            .map(|r| r.name.as_str())
            .unwrap_or("?");
        draw_rectangle(12.0, 10.0, 380.0, 34.0, Color::new(0.0, 0.0, 0.0, 0.55));
        draw_ui_text_ex(
            &format!("{} — {}", map.name, region),
            22.0,
            33.0,
            TextStyle::new(18.0, dark::TEXT_BRIGHT).params(),
        );
        draw_rectangle(
            12.0,
            LOGICAL_HEIGHT - 40.0,
            700.0,
            30.0,
            Color::new(0.0, 0.0, 0.0, 0.45),
        );
        draw_ui_text_ex(
            "WASD/arrows move · Space interact · Esc menu",
            22.0,
            LOGICAL_HEIGHT - 19.0,
            TextStyle::new(15.0, dark::TEXT_DIM).params(),
        );
    }
}

fn npc_dialogue(npc: &NpcDef) -> DialogueBox {
    DialogueBox {
        name: npc.name.clone(),
        lines: npc.lines.clone(),
        index: 0,
    }
}

fn held_direction() -> Option<(i32, i32)> {
    if is_key_down(KeyCode::Left) || is_key_down(KeyCode::A) {
        Some((-1, 0))
    } else if is_key_down(KeyCode::Right) || is_key_down(KeyCode::D) {
        Some((1, 0))
    } else if is_key_down(KeyCode::Up) || is_key_down(KeyCode::W) {
        Some((0, -1))
    } else if is_key_down(KeyCode::Down) || is_key_down(KeyCode::S) {
        Some((0, 1))
    } else {
        None
    }
}

/// Fliers earn their keep as overworld pace (`creature.md` §2.4).
fn step_time(data: &GameData, session: &GameSession) -> f32 {
    let has_flier = session
        .profile
        .roster
        .party_members()
        .any(|c| c.species(data).natural_flight);
    if has_flier {
        0.10
    } else {
        0.15
    }
}

fn roll_encounter(map: &MapDef, data: &GameData, rng: &mut Rng) -> Option<Vec<UnitSpec>> {
    let total: u32 = map.encounters.iter().map(|e| e.weight).sum();
    if total == 0 {
        return None;
    }
    let mut roll = (rng.next_u64() % total as u64) as u32;
    let entry = map.encounters.iter().find(|e| {
        if roll < e.weight {
            true
        } else {
            roll -= e.weight;
            false
        }
    })?;
    let count = entry.min + (rng.below((entry.max - entry.min + 1) as usize) as u32);
    let species = data.species.get(&entry.species)?;
    Some(
        (0..count)
            .map(|i| UnitSpec {
                species_id: entry.species.clone(),
                name: if count > 1 {
                    format!("wild {} {}", species.name, i + 1)
                } else {
                    format!("wild {}", species.name)
                },
                side: Side::Enemy,
                creature_id: None,
                bond: 0.0,
                stance: Stance::Aggressive,
                grafts: Vec::new(),
            })
            .collect(),
    )
}

fn draw_tile(kind: TileKind, px: f32, py: f32, tx: i32, ty: i32) {
    // Deterministic per-tile jitter for texture without an RNG.
    let h = ((tx.wrapping_mul(73_856_093) ^ ty.wrapping_mul(19_349_663)) as u32 >> 8) as f32
        / 16_777_216.0;
    match kind {
        TileKind::Ground => {
            draw_rectangle(px, py, TILE, TILE, Color::new(0.16, 0.19, 0.13, 1.0));
            if h > 0.8 {
                draw_circle(
                    px + TILE * 0.3,
                    py + TILE * 0.6,
                    2.0,
                    Color::new(0.20, 0.24, 0.16, 1.0),
                );
            }
        }
        TileKind::Path => {
            draw_rectangle(px, py, TILE, TILE, Color::new(0.26, 0.22, 0.16, 1.0));
        }
        TileKind::Grass => {
            draw_rectangle(px, py, TILE, TILE, Color::new(0.11, 0.23, 0.12, 1.0));
            let sway = h * 6.0;
            draw_line(
                px + 8.0 + sway,
                py + TILE - 6.0,
                px + 10.0 + sway,
                py + 10.0,
                2.0,
                Color::new(0.16, 0.33, 0.17, 1.0),
            );
            draw_line(
                px + 24.0 + sway,
                py + TILE - 6.0,
                px + 26.0 + sway,
                py + 14.0,
                2.0,
                Color::new(0.14, 0.30, 0.15, 1.0),
            );
        }
        TileKind::Tree => {
            draw_rectangle(px, py, TILE, TILE, Color::new(0.09, 0.13, 0.09, 1.0));
            draw_circle(
                px + TILE * 0.5,
                py + TILE * 0.4,
                TILE * 0.38,
                Color::new(0.07, 0.17, 0.10, 1.0),
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

fn draw_dialogue(dialog: &DialogueBox) {
    let rect = Rect::new(60.0, LOGICAL_HEIGHT - 170.0, LOGICAL_WIDTH - 120.0, 130.0);
    draw_surface(
        rect,
        &SurfaceStyle::new(Color::new(0.07, 0.08, 0.10, 0.97))
            .with_border(1.5, Color::new(0.5, 0.55, 0.65, 0.8)),
    );
    draw_ui_text_ex(
        &dialog.name,
        rect.x + 20.0,
        rect.y + 30.0,
        TextStyle::new(18.0, Color::new(0.85, 0.78, 0.6, 1.0)).params(),
    );
    if let Some(line) = dialog.lines.get(dialog.index) {
        draw_text_block(
            line,
            rect.x + 20.0,
            rect.y + 46.0,
            rect.w - 40.0,
            rect.h - 60.0,
            17.0,
            5.0,
            dark::TEXT,
        );
    }
    draw_ui_text_ex(
        "[Space] ▼",
        rect.right() - 100.0,
        rect.bottom() - 14.0,
        TextStyle::new(14.0, dark::TEXT_DIM).params(),
    );
}
