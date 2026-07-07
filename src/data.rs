//! Embedded game data: definitions loaded from `assets/data/*.json`.

pub mod balance;
pub mod factory;
pub mod graftware;
pub mod settlement;
pub mod species;
pub mod world;

use balance::BalanceConfig;
use factory::FactoryDef;
use graftware::GraftwareDef;
use macroquad_toolkit::assets::TextureConfig;
use macroquad_toolkit::data_loader::{
    load_embedded_json, load_embedded_json_labeled, DataRegistry,
};
use serde::{Deserialize, Serialize};
use settlement::SettlementDef;
use species::SpeciesDef;
use world::{MapKind, WorldDef};

const GAME_CONFIG_JSON: &str = include_str!("../assets/data/game_config.json");
const BALANCE_JSON: &str = include_str!("../assets/data/balance.json");
const SPECIES_JSON: &str = include_str!("../assets/data/species.json");
const GRAFTWARE_JSON: &str = include_str!("../assets/data/graftware.json");
const WORLD_JSON: &str = include_str!("../assets/data/world.json");
const SETTLEMENTS_JSON: &str = include_str!("../assets/data/settlements.json");
const FACTORIES_JSON: &str = include_str!("../assets/data/factories.json");
/// Per-region content packs — separate files so regions can be authored
/// independently; merged into the same registries at load.
const REGION_PACKS: [(&str, &str); 5] = [
    (
        "mirrormere",
        include_str!("../assets/data/regions/mirrormere.json"),
    ),
    (
        "stormcap",
        include_str!("../assets/data/regions/stormcap.json"),
    ),
    (
        "ashvein",
        include_str!("../assets/data/regions/ashvein.json"),
    ),
    (
        "sporefen",
        include_str!("../assets/data/regions/sporefen.json"),
    ),
    (
        "bonewhite",
        include_str!("../assets/data/regions/bonewhite.json"),
    ),
];

/// One region's authored content, merged into the world at load.
#[derive(Debug, Clone, Deserialize)]
struct RegionPack {
    #[serde(default)]
    maps: Vec<world::MapDef>,
    #[serde(default)]
    settlements: Vec<SettlementDef>,
    #[serde(default)]
    factories: Vec<FactoryDef>,
}
const TEXTURE_MANIFEST_JSON: &str = include_str!("../assets/data/texture_manifest.json");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameConfig {
    pub game_name: String,
    pub display_name: String,
    pub save_slot: String,
    pub version: String,
}

#[derive(Debug, Clone)]
pub struct GameData {
    pub config: GameConfig,
    pub balance: BalanceConfig,
    pub species: DataRegistry<SpeciesDef>,
    pub graftware: DataRegistry<GraftwareDef>,
    pub world: WorldDef,
    pub settlements: DataRegistry<SettlementDef>,
    pub factories: DataRegistry<FactoryDef>,
    pub texture_manifest: Vec<TextureConfig>,
}

impl GameData {
    pub fn load() -> Result<Self, String> {
        let config = load_embedded_json_labeled("game_config", GAME_CONFIG_JSON)?;
        let balance = load_embedded_json_labeled("balance", BALANCE_JSON)?;
        let species = DataRegistry::from_embedded_json(SPECIES_JSON, "id")?;
        let graftware = DataRegistry::from_embedded_json(GRAFTWARE_JSON, "id")?;
        let mut world: WorldDef = load_embedded_json_labeled("world", WORLD_JSON)?;
        let mut settlements: DataRegistry<SettlementDef> =
            DataRegistry::from_embedded_json(SETTLEMENTS_JSON, "id")?;
        let mut factories: DataRegistry<FactoryDef> =
            DataRegistry::from_embedded_json(FACTORIES_JSON, "id")?;
        let texture_manifest = load_embedded_json(TEXTURE_MANIFEST_JSON)?;

        for (name, json) in REGION_PACKS {
            let pack: RegionPack = load_embedded_json_labeled(name, json)?;
            world.maps.extend(pack.maps);
            for s in pack.settlements {
                settlements.insert(s.id.clone(), s);
            }
            for f in pack.factories {
                factories.insert(f.id.clone(), f);
            }
        }

        // Merge factory floors into the world map list so traversal, warps,
        // and encounters work identically inside a Gestarium.
        for (_, factory) in factories.iter() {
            for floor in &factory.floors {
                let mut floor = floor.clone();
                floor.kind = MapKind::Factory;
                floor.factory_id = Some(factory.id.clone());
                world.maps.push(floor);
            }
        }

        Ok(Self {
            config,
            balance,
            species,
            graftware,
            world,
            settlements,
            factories,
            texture_manifest,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_data_loads() {
        let data = GameData::load().unwrap();
        assert_eq!(data.config.game_name, "iron_fauna");
        assert!(!data.species.is_empty());
        assert!(!data.graftware.is_empty());
    }
}
