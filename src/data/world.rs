//! World definitions: regions and connected overworld tile maps
//! (`game_design.md` §8 — Pokémon-style connected 2D maps).
//!
//! Maps are authored as ASCII rows in `assets/data/world.json` (and factory
//! floors in `factories.json`):
//! `#` tree/wall · `.` ground · `,` path · `g` tall grass (encounters) ·
//! `~` water · `^` rocks · `s` settlement door · `D` gestarium door ·
//! `=` deck plate · `v` vat spill (factory encounters) · `V` gestation vat ·
//! `H` the factory heart.

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
    DeckPlate,
    VatSpill,
    Vat,
    Heart,
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
            '=' => Some(TileKind::DeckPlate),
            'v' => Some(TileKind::VatSpill),
            'V' => Some(TileKind::Vat),
            'H' => Some(TileKind::Heart),
            _ => None,
        }
    }

    pub fn walkable(self) -> bool {
        !matches!(
            self,
            TileKind::Tree | TileKind::Water | TileKind::Rock | TileKind::Vat | TileKind::Heart
        )
    }

    /// Tiles that can trigger encounters when stepped on.
    pub fn encounter_prone(self) -> bool {
        matches!(self, TileKind::Grass | TileKind::VatSpill)
    }
}

/// What kind of map this is — controls palette and encounter arming.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum MapKind {
    #[default]
    Overworld,
    Factory,
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

/// A condition gating a dialogue rule. Empty fields don't constrain.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DialogueCond {
    /// All of these story flags must be set.
    #[serde(default)]
    pub flags_all: Vec<String>,
    /// None of these story flags may be set.
    #[serde(default)]
    pub flags_none: Vec<String>,
    /// This factory's heart must be defeated.
    #[serde(default)]
    pub heart_defeated: Option<String>,
    /// This factory must carry this verdict ("Purge"|"Reseed"|"Bind").
    #[serde(default)]
    pub verdict: Option<(String, String)>,
    /// This factory must (true) / must not (false) be in relapse.
    #[serde(default)]
    pub relapsed: Option<(String, bool)>,
}

/// One conditional dialogue variant. The first rule whose condition passes
/// plays; author fallbacks last.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueRule {
    #[serde(default)]
    pub when: Option<DialogueCond>,
    pub lines: Vec<String>,
    /// Story flags set after the dialogue closes.
    #[serde(default)]
    pub set_flags: Vec<String>,
    /// One-shot rewards granted when the dialogue closes.
    #[serde(default)]
    pub give_scrip: i64,
    #[serde(default)]
    pub give_grafts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcDef {
    pub id: String,
    pub x: i32,
    pub y: i32,
    pub name: String,
    /// Simple unconditional lines (legacy shorthand for one fallback rule).
    #[serde(default)]
    pub lines: Vec<String>,
    /// Conditional dialogue, checked in order before falling back to `lines`.
    #[serde(default)]
    pub dialogue: Vec<DialogueRule>,
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
    #[serde(default)]
    pub kind: MapKind,
    /// Set on factory floors: which Gestarium this floor belongs to.
    #[serde(default)]
    pub factory_id: Option<String>,
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
            assert!(
                world.region(&map.region).is_some(),
                "{}: bad region",
                map.id
            );
            for warp in &map.warps {
                let target = world.map(&warp.to_map);
                assert!(
                    target.is_some(),
                    "{}: warp to unknown map {}",
                    map.id,
                    warp.to_map
                );
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
            for npc in &map.npcs {
                assert!(
                    !npc.lines.is_empty() || !npc.dialogue.is_empty(),
                    "{}: NPC {} has nothing to say",
                    map.id,
                    npc.id
                );
                for rule in &npc.dialogue {
                    assert!(!rule.lines.is_empty(), "{}: empty rule", npc.id);
                    for def in &rule.give_grafts {
                        assert!(
                            data.graftware.contains(def),
                            "{}: unknown reward graft {}",
                            npc.id,
                            def
                        );
                    }
                    if let Some(cond) = &rule.when {
                        if let Some((factory, verdict)) = &cond.verdict {
                            assert!(
                                data.factories.contains(factory),
                                "{}: unknown factory {}",
                                npc.id,
                                factory
                            );
                            assert!(
                                matches!(verdict.as_str(), "Purge" | "Reseed" | "Bind"),
                                "{}: bad verdict name {}",
                                npc.id,
                                verdict
                            );
                        }
                        if let Some(factory) = &cond.heart_defeated {
                            assert!(
                                data.factories.contains(factory),
                                "{}: unknown factory {}",
                                npc.id,
                                factory
                            );
                        }
                        if let Some((factory, _)) = &cond.relapsed {
                            assert!(
                                data.factories.contains(factory),
                                "{}: unknown factory {}",
                                npc.id,
                                factory
                            );
                        }
                    }
                }
            }
        }
    }
}
