//! World definitions: regions and connected overworld tile maps
//! (`game_design.md` §8 — Pokémon-style connected 2D maps).
//!
//! Maps are authored as ASCII rows in `assets/data/world.json`:
//! `#` tree/wall · `.` ground · `,` path · `g` tall grass (encounters) ·
//! `~` water · `^` rocks · `s` settlement door · `D` gestarium door.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileKind {
    Ground,
    Path,
    Grass,
    Tree,
    Water,
    Rock,
    SettlementDoor,
    GestariumDoor,
}

impl TileKind {
    pub fn from_char(c: char) -> Option<TileKind> {
        match c {
            '.' => Some(TileKind::Ground),
            ',' => Some(TileKind::Path),
            'g' => Some(TileKind::Grass),
            '#' => Some(TileKind::Tree),
            '~' => Some(TileKind::Water),
            '^' => Some(TileKind::Rock),
            's' => Some(TileKind::SettlementDoor),
            'D' => Some(TileKind::GestariumDoor),
            _ => None,
        }
    }

    pub fn walkable(self) -> bool {
        !matches!(self, TileKind::Tree | TileKind::Water | TileKind::Rock)
    }

    pub fn encounter_prone(self) -> bool {
        self == TileKind::Grass
    }
}

/// How a region currently stands — evolves with factory verdicts (§9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RegionMood {
    /// Factory still active: dangerous wilds, wary settlements.
    Threatened,
    /// Purged: safe but dead ground.
    Dead,
    /// Reseeded: reviving, lush — and quietly at risk.
    Reviving,
    /// Bound: the player runs the machine.
    Claimed,
    /// Relapsed: militarized, hostile again.
    Relapsed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionDef {
    pub id: String,
    pub name: String,
    pub description: String,
    /// Content tier band for wilds and factory output.
    pub tier: u32,
    pub gestarium_id: String,
    pub biomes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncounterEntry {
    pub species: String,
    pub weight: u32,
    /// Pack size bounds (inclusive).
    #[serde(default = "one")]
    pub min: u32,
    #[serde(default = "one")]
    pub max: u32,
}

fn one() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WarpDef {
    pub x: i32,
    pub y: i32,
    pub to_map: String,
    pub to_x: i32,
    pub to_y: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcDef {
    pub id: String,
    pub x: i32,
    pub y: i32,
    pub name: String,
    /// Dialogue lines shown in order on interaction.
    pub lines: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapDef {
    pub id: String,
    pub name: String,
    pub region: String,
    pub rows: Vec<String>,
    pub spawn_x: i32,
    pub spawn_y: i32,
    /// Chance per grass step of a wild encounter.
    #[serde(default)]
    pub encounter_rate: f32,
    #[serde(default)]
    pub encounters: Vec<EncounterEntry>,
    #[serde(default)]
    pub warps: Vec<WarpDef>,
    #[serde(default)]
    pub npcs: Vec<NpcDef>,
    /// Which settlement the `s` doors open (placeholder: outfit bench).
    #[serde(default)]
    pub settlement: Option<String>,
}

impl MapDef {
    pub fn width(&self) -> i32 {
        self.rows.first().map(|r| r.chars().count()).unwrap_or(0) as i32
    }

    pub fn height(&self) -> i32 {
        self.rows.len() as i32
    }

    pub fn tile(&self, x: i32, y: i32) -> TileKind {
        if x < 0 || y < 0 || y >= self.height() {
            return TileKind::Tree;
        }
        self.rows[y as usize]
            .chars()
            .nth(x as usize)
            .and_then(TileKind::from_char)
            .unwrap_or(TileKind::Tree)
    }

    pub fn walkable(&self, x: i32, y: i32) -> bool {
        self.tile(x, y).walkable() && !self.npcs.iter().any(|n| n.x == x && n.y == y)
    }

    pub fn warp_at(&self, x: i32, y: i32) -> Option<&WarpDef> {
        self.warps.iter().find(|w| w.x == x && w.y == y)
    }

    pub fn npc_at(&self, x: i32, y: i32) -> Option<&NpcDef> {
        self.npcs.iter().find(|n| n.x == x && n.y == y)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldDef {
    pub start_map: String,
    pub regions: Vec<RegionDef>,
    pub maps: Vec<MapDef>,
}

impl WorldDef {
    pub fn map(&self, id: &str) -> Option<&MapDef> {
        self.maps.iter().find(|m| m.id == id)
    }

    pub fn region(&self, id: &str) -> Option<&RegionDef> {
        self.regions.iter().find(|r| r.id == id)
    }
}

#[cfg(test)]
mod tests {
    use crate::data::GameData;

    #[test]
    fn world_is_internally_consistent() {
        let data = GameData::load().unwrap();
        let world = &data.world;
        assert!(world.map(&world.start_map).is_some(), "start map missing");

        for map in &world.maps {
            assert!(map.height() > 0 && map.width() > 0, "{}: empty map", map.id);
            for (y, row) in map.rows.iter().enumerate() {
                assert_eq!(
                    row.chars().count() as i32,
                    map.width(),
                    "{}: ragged row {}",
                    map.id,
                    y
                );
                for (x, c) in row.chars().enumerate() {
                    assert!(
                        crate::data::world::TileKind::from_char(c).is_some(),
                        "{}: unknown tile '{}' at {},{}",
                        map.id,
                        c,
                        x,
                        y
                    );
                }
            }
            assert!(
                map.walkable(map.spawn_x, map.spawn_y),
                "{}: spawn blocked",
                map.id
            );
            assert!(world.region(&map.region).is_some(), "{}: bad region", map.id);
            for warp in &map.warps {
                let target = world.map(&warp.to_map);
                assert!(target.is_some(), "{}: warp to unknown map {}", map.id, warp.to_map);
                assert!(
                    target.unwrap().walkable(warp.to_x, warp.to_y),
                    "{}: warp lands blocked in {}",
                    map.id,
                    warp.to_map
                );
            }
            for e in &map.encounters {
                assert!(
                    data.species.contains(&e.species),
                    "{}: unknown species {}",
                    map.id,
                    e.species
                );
                assert!(e.min >= 1 && e.min <= e.max, "{}: bad pack bounds", map.id);
            }
            if map.encounter_rate > 0.0 {
                assert!(!map.encounters.is_empty(), "{}: rate but no table", map.id);
            }
        }
    }
}
