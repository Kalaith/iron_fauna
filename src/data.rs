//! Embedded game data: definitions loaded from `assets/data/*.json`.

pub mod balance;
pub mod graftware;
pub mod species;

use balance::BalanceConfig;
use graftware::GraftwareDef;
use macroquad_toolkit::assets::TextureConfig;
use macroquad_toolkit::data_loader::{
    load_embedded_json, load_embedded_json_labeled, DataRegistry,
};
use serde::{Deserialize, Serialize};
use species::SpeciesDef;

const GAME_CONFIG_JSON: &str = include_str!("../assets/data/game_config.json");
const BALANCE_JSON: &str = include_str!("../assets/data/balance.json");
const SPECIES_JSON: &str = include_str!("../assets/data/species.json");
const GRAFTWARE_JSON: &str = include_str!("../assets/data/graftware.json");
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
    pub texture_manifest: Vec<TextureConfig>,
}

impl GameData {
    pub fn load() -> Result<Self, String> {
        let config = load_embedded_json_labeled("game_config", GAME_CONFIG_JSON)?;
        let balance = load_embedded_json_labeled("balance", BALANCE_JSON)?;
        let species = DataRegistry::from_embedded_json(SPECIES_JSON, "id")?;
        let graftware = DataRegistry::from_embedded_json(GRAFTWARE_JSON, "id")?;
        let texture_manifest = load_embedded_json(TEXTURE_MANIFEST_JSON)?;

        Ok(Self {
            config,
            balance,
            species,
            graftware,
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
