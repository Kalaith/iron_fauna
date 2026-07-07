//! Overworld traversal: connected tile maps, grid movement, wild encounters,
//! NPC dialogue, and doors into settlements and Gestaria (`game_design.md` §8).

use crate::combat::unit::UnitSpec;
use crate::combat::{Side, Stance};
use crate::data::world::{DialogueRule, DoorTarget, MapDef, MapKind, TileKind};
use crate::data::GameData;
use crate::model::story;
use crate::model::warunit::war_unit_grafts;
use crate::model::worldstate::{RegionMood, WorldState};
use crate::state::GameSession;
use crate::ui::{LOGICAL_HEIGHT, LOGICAL_WIDTH};
use crate::util::Rng;
use macroquad::prelude::*;
use macroquad_toolkit::assets::AssetManager;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::draw_ui_text_ex;

mod tiles;
use tiles::draw_tile;

const TILE: f32 = 40.0;

pub enum OverworldResult {
    Continue,
    /// Stepped through a settlement door — open the facility it leads to.
    OpenSettlement(DoorTarget),
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
    /// Drawn position in tile units — glides toward the logical (grid) tile so
    /// movement and camera read as continuous while the sim stays grid-locked.
    visual_x: f32,
    visual_y: f32,
    dialogue: Option<DialogueBox>,
    rng: Rng,
}

impl OverworldScreen {
    pub fn new(session: &GameSession) -> Self {
        Self {
            move_timer: 0.0,
            facing: (0, 1),
            visual_x: session.location.x as f32,
            visual_y: session.location.y as f32,
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

        // Glide the drawn position toward the logical tile every frame. Speed is
        // one tile per step interval, so the sprite arrives exactly as the next
        // grid step unlocks — holding a direction reads as one smooth walk.
        let glide = dt / step_time(data, session);
        self.visual_x = approach(self.visual_x, session.location.x as f32, glide);
        self.visual_y = approach(self.visual_y, session.location.y as f32, glide);

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
                        self.snap_visual(session);
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
                // On a diagonal, refuse to squeeze between two walls — both
                // orthogonal neighbours blocked means there's no real gap.
                let cut_corner = dir.0 != 0
                    && dir.1 != 0
                    && !map.walkable(session.location.x + dir.0, session.location.y)
                    && !map.walkable(session.location.x, session.location.y + dir.1);
                if map.walkable(nx, ny) && !cut_corner {
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
                        self.snap_visual(session);
                        return OverworldResult::Continue;
                    }
                    let tile = map.tile(nx, ny);
                    if tile == TileKind::SettlementDoor {
                        let target = map
                            .door_at(nx, ny)
                            .map(|d| d.target)
                            .unwrap_or(DoorTarget::Hub);
                        return OverworldResult::OpenSettlement(target);
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

    /// Teleports (warps, door transits) must not glide across the gap — pin the
    /// drawn position to the new tile so the next map starts settled.
    fn snap_visual(&mut self, session: &GameSession) {
        self.visual_x = session.location.x as f32;
        self.visual_y = session.location.y as f32;
    }

    pub fn draw(&self, data: &GameData, session: &GameSession, assets: &AssetManager) {
        let Some(map) = data.world.map(&session.location.map_id) else {
            return;
        };

        // Camera: center on the (gliding) player in tile units, clamped to the
        // map bounds. Kept as a float so scrolling tracks the smooth position.
        let view_w = LOGICAL_WIDTH / TILE;
        let view_h = LOGICAL_HEIGHT / TILE;
        let cam_x =
            (self.visual_x - view_w * 0.5).clamp(0.0, (map.width() as f32 - view_w).max(0.0));
        let cam_y =
            (self.visual_y - view_h * 0.5).clamp(0.0, (map.height() as f32 - view_h).max(0.0));

        let mood = match map.kind {
            MapKind::Factory => RegionMood::Threatened,
            MapKind::Overworld => session.world_state.region_mood(data, &map.region),
        };
        clear_background(if map.kind == MapKind::Factory {
            Color::new(0.04, 0.04, 0.05, 1.0)
        } else {
            Color::new(0.05, 0.06, 0.05, 1.0)
        });
        let first_tx = cam_x.floor() as i32;
        let first_ty = cam_y.floor() as i32;
        for ty in first_ty..(first_ty + view_h.ceil() as i32 + 2).min(map.height()) {
            for tx in first_tx..(first_tx + view_w.ceil() as i32 + 2).min(map.width()) {
                let px = (tx as f32 - cam_x) * TILE;
                let py = (ty as f32 - cam_y) * TILE;
                draw_tile(
                    assets,
                    map.tile(tx, ty),
                    vec2(px, py),
                    (tx, ty),
                    map.kind,
                    mood,
                );
            }
        }

        // Building signs: label each door so the town reads as distinct shops.
        for door in &map.doors {
            let Some(label) = &door.label else { continue };
            let px = (door.x as f32 - cam_x) * TILE + TILE * 0.5;
            let py = (door.y as f32 - cam_y) * TILE;
            if px < -TILE || py < -TILE || px > LOGICAL_WIDTH + TILE || py > LOGICAL_HEIGHT + TILE {
                continue;
            }
            let w = label.len() as f32 * 7.5 + 12.0;
            draw_rectangle(
                px - w * 0.5,
                py - 20.0,
                w,
                17.0,
                Color::new(0.0, 0.0, 0.0, 0.6),
            );
            draw_text_centered_in_box_ex(
                label,
                px - w * 0.5,
                py - 20.0,
                w,
                17.0,
                TextStyle::new(13.0, Color::new(0.90, 0.82, 0.62, 1.0)),
            );
        }

        for npc in &map.npcs {
            let px = (npc.x as f32 - cam_x) * TILE + TILE * 0.5;
            let py = (npc.y as f32 - cam_y) * TILE + TILE * 0.5;
            if px < -TILE || py < -TILE || px > LOGICAL_WIDTH + TILE || py > LOGICAL_HEIGHT + TILE {
                continue;
            }
            draw_circle(px, py - 6.0, 9.0, Color::new(0.80, 0.70, 0.55, 1.0));
            draw_rectangle(px - 7.0, py, 14.0, 14.0, Color::new(0.45, 0.40, 0.55, 1.0));
        }

        // Player, drawn at the smooth (gliding) position.
        let px = (self.visual_x - cam_x) * TILE + TILE * 0.5;
        let py = (self.visual_y - cam_y) * TILE + TILE * 0.5;
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

        // Quest tracker: the accepted objective, or a turn-in prompt.
        if let Some(line) = crate::model::quest::tracker_line(session, data) {
            let gold = Color::new(0.95, 0.82, 0.45, 1.0);
            draw_rectangle(12.0, 48.0, 380.0, 26.0, Color::new(0.0, 0.0, 0.0, 0.5));
            draw_rectangle(12.0, 48.0, 4.0, 26.0, gold);
            draw_ui_text_ex(
                &format!("Quest: {}", line),
                24.0,
                66.0,
                TextStyle::new(15.0, gold).params(),
            );
        }

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

/// Combine held keys into an 8-way step; opposite keys cancel so pressing
/// left+right (or up+down) leaves that axis neutral.
fn held_direction() -> Option<(i32, i32)> {
    let mut dx = 0;
    let mut dy = 0;
    if is_key_down(KeyCode::Left) || is_key_down(KeyCode::A) {
        dx -= 1;
    }
    if is_key_down(KeyCode::Right) || is_key_down(KeyCode::D) {
        dx += 1;
    }
    if is_key_down(KeyCode::Up) || is_key_down(KeyCode::W) {
        dy -= 1;
    }
    if is_key_down(KeyCode::Down) || is_key_down(KeyCode::S) {
        dy += 1;
    }
    if dx == 0 && dy == 0 {
        None
    } else {
        Some((dx, dy))
    }
}

/// Move `cur` toward `target` by at most `max_delta`, snapping on arrival.
fn approach(cur: f32, target: f32, max_delta: f32) -> f32 {
    let d = target - cur;
    if d.abs() <= max_delta {
        target
    } else {
        cur + d.signum() * max_delta
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
