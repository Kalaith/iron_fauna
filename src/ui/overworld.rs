//! Overworld traversal: connected tile maps, grid movement, wild encounters,
//! NPC dialogue, and doors into settlements and Gestaria (`game_design.md` §8).

use crate::combat::unit::UnitSpec;
use crate::combat::{Side, Stance};
use crate::data::world::{DialogueRule, MapDef, MapKind, TileKind};
use crate::data::GameData;
use crate::model::story;
use crate::model::warunit::war_unit_grafts;
use crate::model::worldstate::{RegionMood, WorldState};
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
    /// Walked into a wild pack (or a factory patrol).
    StartEncounter(Vec<UnitSpec>),
    /// Interacted with a factory heart.
    HeartInteract(String),
    BackToMenu,
}

struct DialogueBox {
    name: String,
    lines: Vec<String>,
    index: usize,
    /// Effects applied when the dialogue closes.
    on_close: Option<DialogueRule>,
}

impl DialogueBox {
    fn plain(name: &str, lines: Vec<String>) -> Self {
        Self {
            name: name.to_owned(),
            lines,
            index: 0,
            on_close: None,
        }
    }
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
                    let closed = self.dialogue.take();
                    if let Some(rule) = closed.and_then(|d| d.on_close) {
                        let notes = story::apply_dialogue_effects(&rule, session, data);
                        if !notes.is_empty() {
                            self.dialogue = Some(DialogueBox::plain("Received", notes));
                        }
                    }
                }
            }
            return OverworldResult::Continue;
        }

        // Interact with whatever we're facing.
        if is_key_pressed(KeyCode::Space) || is_key_pressed(KeyCode::Enter) {
            let fx = session.location.x + self.facing.0;
            let fy = session.location.y + self.facing.1;
            if let Some(npc) = map.npc_at(fx, fy) {
                if let Some(selection) = story::select_dialogue(npc, session) {
                    self.dialogue = Some(DialogueBox {
                        name: npc.name.clone(),
                        lines: selection.lines.to_vec(),
                        index: 0,
                        on_close: selection.rule.cloned(),
                    });
                }
                return OverworldResult::Continue;
            }
            match map.tile(fx, fy) {
                TileKind::GestariumDoor => {
                    // Doors with a warp behind them open; the rest stay shut.
                    if let Some(warp) = map.warp_at(fx, fy) {
                        session.location.map_id = warp.to_map.clone();
                        session.location.x = warp.to_x;
                        session.location.y = warp.to_y;
                    } else {
                        self.dialogue = Some(DialogueBox::plain(
                            "Sealed Doors",
                            vec![
                                "Warm air breathes through the seam. Deep in the hum of the earth, something is still growing.".to_owned(),
                                "The doors do not answer. Not yet.".to_owned(),
                            ],
                        ));
                    }
                    return OverworldResult::Continue;
                }
                TileKind::Heart => {
                    if let Some(factory_id) = &map.factory_id {
                        return OverworldResult::HeartInteract(factory_id.clone());
                    }
                }
                _ => {}
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

                    // Untended reseeded regions slide toward relapse (§9.1);
                    // the tip-over arrives as a story beat, not a stat.
                    if let Some(factory_id) = session.world_state.tick_relapse(
                        data.balance.world.relapse_per_step,
                        data.balance.world.relapse_invested_mult,
                    ) {
                        let name = data
                            .factories
                            .get(&factory_id)
                            .map(|f| f.name.clone())
                            .unwrap_or(factory_id);
                        self.dialogue = Some(DialogueBox::plain(
                            "Word on the road",
                            vec![
                                format!("Travellers say the land around {} is wrong again. Grafted shapes drilling in the fields you brought back to life.", name),
                                "You remember planting that seed. Someone is watering it with the old poison.".to_owned(),
                                "The heart will have a new keeper. Go and meet what you made possible.".to_owned(),
                            ],
                        ));
                    }

                    if let Some(warp) = map.warp_at(nx, ny) {
                        session.location.map_id = warp.to_map.clone();
                        session.location.x = warp.to_x;
                        session.location.y = warp.to_y;
                        return OverworldResult::Continue;
                    }
                    let tile = map.tile(nx, ny);
                    if tile == TileKind::SettlementDoor {
                        return OverworldResult::OpenSettlement;
                    }
                    if tile.encounter_prone() {
                        let rate = effective_encounter_rate(map, data, session);
                        if self.rng.chance(rate) {
                            // Relapsed regions field armed patrols in the open.
                            let mood = session.world_state.region_mood(data, &map.region);
                            let armed =
                                map.kind == MapKind::Factory || mood == RegionMood::Relapsed;
                            if let Some(pack) = roll_encounter(map, data, &mut self.rng, armed) {
                                return OverworldResult::StartEncounter(pack);
                            }
                        }
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

        let mood = match map.kind {
            MapKind::Factory => RegionMood::Threatened,
            MapKind::Overworld => session.world_state.region_mood(data, &map.region),
        };
        clear_background(if map.kind == MapKind::Factory {
            Color::new(0.04, 0.04, 0.05, 1.0)
        } else {
            Color::new(0.05, 0.06, 0.05, 1.0)
        });
        for ty in cam_y..(cam_y + view_h + 1).min(map.height()) {
            for tx in cam_x..(cam_x + view_w + 1).min(map.width()) {
                let px = (tx - cam_x) as f32 * TILE;
                let py = (ty - cam_y) as f32 * TILE;
                draw_tile(map.tile(tx, ty), px, py, tx, ty, map.kind, mood);
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

        self.draw_hud(data, session, map);
        if let Some(dialog) = &self.dialogue {
            draw_dialogue(dialog);
        }
    }

    fn draw_hud(&self, data: &GameData, session: &GameSession, map: &MapDef) {
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

        // Objective hint: what this region wants of the player right now.
        let (hint, color) = region_objective(data, session, map);
        let hw = hint.len() as f32 * 8.0 + 40.0;
        draw_rectangle(
            LOGICAL_WIDTH * 0.5 - hw * 0.5,
            10.0,
            hw,
            30.0,
            Color::new(0.0, 0.0, 0.0, 0.55),
        );
        draw_text_centered_in_box_ex(
            &hint,
            LOGICAL_WIDTH * 0.5 - hw * 0.5,
            10.0,
            hw,
            30.0,
            TextStyle::new(16.0, color),
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

/// The current region's live objective, derived from world state — turns the
/// open world into something legible without hard gates.
fn region_objective(data: &GameData, session: &GameSession, map: &MapDef) -> (String, Color) {
    let amber = Color::new(0.95, 0.82, 0.45, 1.0);
    let red = Color::new(0.95, 0.45, 0.4, 1.0);
    let green = Color::new(0.55, 0.85, 0.55, 1.0);
    let grey = Color::new(0.7, 0.72, 0.75, 1.0);
    let purple = Color::new(0.72, 0.6, 0.9, 1.0);

    if map.kind == MapKind::Factory {
        if let Some(fid) = &map.factory_id {
            let f = session.world_state.factory(fid);
            if !f.heart_defeated {
                return ("Descend to the heart and silence it.".to_owned(), amber);
            }
            return (
                "The vats are still. Nothing left to fight here.".to_owned(),
                grey,
            );
        }
    }

    let Some(region) = data.world.region(&map.region) else {
        return (String::new(), grey);
    };
    let f = session.world_state.factory(&region.gestarium_id);
    let fname = data
        .factories
        .get(&region.gestarium_id)
        .map(|d| d.name.clone())
        .unwrap_or_else(|| "the factory".to_owned());

    if !f.heart_defeated {
        (
            format!("{} still births war-units. Raid its heart.", fname),
            amber,
        )
    } else if f.verdict.is_none() {
        (
            format!("{} lies silent — return to pass judgment.", fname),
            amber,
        )
    } else if f.relapsed {
        (
            "This region has RELAPSED. Confront the keeper at the heart.".to_owned(),
            red,
        )
    } else if matches!(f.verdict, Some(crate::model::worldstate::Verdict::Reseed)) && !f.invested {
        (
            "Revived — fund the Watch before prosperity forgets.".to_owned(),
            green,
        )
    } else {
        match f.verdict {
            Some(crate::model::worldstate::Verdict::Purge) => (
                "At peace. Dead peace, but peace. Your verdict holds.".to_owned(),
                grey,
            ),
            Some(crate::model::worldstate::Verdict::Bind) => {
                ("The factory answers to you now.".to_owned(), purple)
            }
            _ => ("Thriving under your watch.".to_owned(), green),
        }
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

/// Encounter richness responds to the region's verdict; dormant factories
/// stop birthing patrols entirely.
fn effective_encounter_rate(map: &MapDef, data: &GameData, session: &GameSession) -> f32 {
    match map.kind {
        MapKind::Factory => {
            let active = map
                .factory_id
                .as_deref()
                .map(|id| session.world_state.factory_active(id))
                .unwrap_or(true);
            if active {
                map.encounter_rate
            } else {
                0.0
            }
        }
        MapKind::Overworld => {
            let mood = session.world_state.region_mood(data, &map.region);
            map.encounter_rate * WorldState::encounter_rate_mult(mood)
        }
    }
}

fn roll_encounter(
    map: &MapDef,
    data: &GameData,
    rng: &mut Rng,
    armed: bool,
) -> Option<Vec<UnitSpec>> {
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
    // Factory-born (and relapse-militarized) units come out already armed.
    let tier = data.world.region(&map.region).map(|r| r.tier).unwrap_or(1);
    Some(
        (0..count)
            .map(|i| {
                let grafts = if armed {
                    war_unit_grafts(species, data, tier, rng)
                } else {
                    Vec::new()
                };
                let label = if armed { "war-unit" } else { "wild" };
                UnitSpec {
                    species_id: entry.species.clone(),
                    name: if count > 1 {
                        format!("{} {} {}", label, species.name, i + 1)
                    } else {
                        format!("{} {}", label, species.name)
                    },
                    side: Side::Enemy,
                    creature_id: None,
                    bond: 0.0,
                    stance: Stance::Aggressive,
                    grafts,
                }
            })
            .collect(),
    )
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

fn draw_tile(
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
