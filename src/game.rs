//! High-level game loop and state transitions.

use crate::combat::engine::Battle;
use crate::combat::unit::UnitSpec;
use crate::combat::{resolve, BattleContext, RiderMods, Side, Stance};
use crate::data::GameData;
use crate::state::{migrate_save_value, GameSession, SaveData};
use crate::ui::battle::{BattleScreen, BattleScreenResult};
use crate::ui::outfit::{OutfitAction, OutfitScreen};
use crate::ui::{self, MenuContext, UiAction};
use macroquad::prelude::*;
use macroquad_toolkit::assets::AssetManager;
use macroquad_toolkit::notifications::{
    NotificationAnchor, NotificationManager, NotificationRenderConfig,
};
use macroquad_toolkit::persistence::{
    load_from_slot_with_migration, save_to_slot_with_version, slot_exists,
};
use macroquad_toolkit::prelude::{begin_virtual_ui_frame, dark, end_virtual_ui_frame};

enum Mode {
    Menu,
    Outfit(OutfitScreen),
    Battle(Box<BattleScreen>),
}

pub struct Game {
    data: GameData,
    session: GameSession,
    #[allow(dead_code)]
    assets: AssetManager,
    notifications: NotificationManager,
    mode: Mode,
    save_exists: bool,
}

impl Game {
    pub async fn new() -> Self {
        let data = GameData::load().unwrap_or_else(|err| {
            panic!("IRON FAUNA embedded data failed to load: {}", err);
        });

        let mut assets = AssetManager::new();
        let placeholder = Image::gen_image_color(16, 16, Color::new(0.75, 0.2, 0.8, 1.0));
        assets.set_placeholder_texture_direct(Texture2D::from_image(&placeholder));
        let _ = assets.load_asset_pack("assets.zip").await;
        let _ = assets.load_texture_configs(&data.texture_manifest).await;

        let session = GameSession::new_game(&data);
        let save_exists = slot_exists(&data.config.game_name, &data.config.save_slot);

        Self {
            data,
            session,
            assets,
            notifications: NotificationManager::new(),
            mode: Mode::Menu,
            save_exists,
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.notifications.update(dt);

        if let Mode::Battle(screen) = &mut self.mode {
            match screen.update(&self.data, dt) {
                BattleScreenResult::Continue => {}
                BattleScreenResult::Finished => self.finish_battle(),
            }
        }
    }

    fn finish_battle(&mut self) {
        let Mode::Battle(screen) = std::mem::replace(&mut self.mode, Mode::Menu) else {
            return;
        };
        let summary = resolve::apply(&mut self.session, &self.data, &screen.battle);
        for line in summary.lines {
            self.notifications.info(line);
        }
    }

    pub fn draw(&mut self) {
        clear_background(dark::BACKGROUND);
        let virtual_ui = begin_virtual_ui_frame(ui::LOGICAL_WIDTH, ui::LOGICAL_HEIGHT);

        let mut actions = Vec::new();
        let mut outfit_actions = Vec::new();
        match &self.mode {
            Mode::Menu => {
                let ctx = MenuContext {
                    data: &self.data,
                    session: &self.session,
                    save_exists: self.save_exists,
                    ui: &virtual_ui,
                };
                actions = ui::draw_main_menu(&ctx);
            }
            Mode::Outfit(screen) => {
                outfit_actions = screen.draw(&self.data, &self.session, &virtual_ui);
            }
            Mode::Battle(screen) => screen.draw(&self.data),
        }
        end_virtual_ui_frame();

        for action in actions {
            self.apply_action(action);
        }
        for action in outfit_actions {
            self.apply_outfit_action(action);
        }

        self.notifications
            .draw_with_config(&NotificationRenderConfig {
                anchor: NotificationAnchor::BottomRight,
                ..Default::default()
            });
    }

    fn apply_action(&mut self, action: UiAction) {
        match action {
            UiAction::NewGame => {
                self.session = GameSession::new_game(&self.data);
                self.notifications.info("A new rider takes the road");
            }
            UiAction::StartDevBattle => self.start_dev_battle(),
            UiAction::OpenOutfit => {
                self.mode = Mode::Outfit(OutfitScreen {
                    selected: self.session.profile.roster.party.first().copied(),
                    selected_slot: None,
                });
            }
            UiAction::Save => self.save_game(),
            UiAction::Load => self.load_game(),
            UiAction::TogglePace => {
                self.session.pace = self.session.pace.toggled();
                self.notifications
                    .info(format!("Pace: {}", self.session.pace.display_name()));
            }
        }
    }

    fn apply_outfit_action(&mut self, action: OutfitAction) {
        let Mode::Outfit(screen) = &mut self.mode else {
            return;
        };
        match action {
            OutfitAction::Back => self.mode = Mode::Menu,
            OutfitAction::SelectCreature(id) => {
                screen.selected = Some(id);
                screen.selected_slot = None;
            }
            OutfitAction::SelectSlot { limb_id, slot } => {
                screen.selected_slot = Some((limb_id, slot));
            }
            OutfitAction::Unequip {
                creature,
                limb_id,
                slot,
            } => {
                if let Some(c) = self.session.profile.roster.creature_mut(creature) {
                    c.unequip(&limb_id, slot);
                }
            }
            OutfitAction::Equip {
                creature,
                limb_id,
                slot,
                item,
            } => {
                let inventory = self.session.profile.inventory.clone();
                if let Some(c) = self.session.profile.roster.creature_mut(creature) {
                    match c.equip(&self.data, &inventory, &limb_id, slot, item) {
                        Ok(()) => screen.selected_slot = None,
                        Err(err) => self.notifications.warning(err.message()),
                    }
                }
            }
            OutfitAction::Repair(item) => {
                if self.session.profile.inventory.repair(&self.data, item) {
                    self.notifications.success("Repaired");
                } else {
                    self.notifications.warning("Can't afford the repair");
                }
            }
            OutfitAction::ToParty(id) => {
                if !self.session.profile.roster.add_to_party(&self.data, id) {
                    self.notifications.warning("No room in the party");
                }
            }
            OutfitAction::ToStorage(id) => {
                if self.session.profile.roster.party.len() <= 1 {
                    self.notifications
                        .warning("The road is no place to walk alone");
                } else {
                    self.session.profile.roster.remove_from_party(id);
                }
            }
        }
    }

    /// Assembles battle specs for the current traveling party.
    fn party_unit_specs(&self) -> Vec<UnitSpec> {
        self.session
            .profile
            .roster
            .party_members()
            .map(|c| {
                let grafts = c
                    .loadout
                    .iter()
                    .filter_map(|m| {
                        let item = self.session.profile.inventory.item(m.item_id)?;
                        item.is_usable().then(|| {
                            (
                                m.limb_id.clone(),
                                m.slot,
                                item.def_id.clone(),
                                Some(m.item_id),
                            )
                        })
                    })
                    .collect();
                UnitSpec {
                    species_id: c.species_id.clone(),
                    name: c.display_name(&self.data).to_owned(),
                    side: Side::Player,
                    creature_id: Some(c.id),
                    bond: c.bond,
                    stance: Stance::Aggressive,
                    grafts,
                }
            })
            .collect()
    }

    /// A deterministic sample encounter until the overworld provides real ones.
    fn start_dev_battle(&mut self) {
        let player = self.party_unit_specs();
        if player.is_empty() {
            self.notifications.warning("No creatures in the party");
            return;
        }
        let wave = self.session.battles_fought % 3;
        let enemy: Vec<UnitSpec> = match wave {
            0 => vec![wild("bumblit"), wild("bumblit")],
            1 => vec![wild("pangol")],
            _ => vec![
                armed(
                    "pangol",
                    vec![
                        ("arm_l", 0, "ember_spitter"),
                        ("back", 0, "basalt_carapace"),
                    ],
                ),
                wild("volpi"),
            ],
        };
        let context = if wave == 2 {
            BattleContext::FactoryDismantle
        } else {
            BattleContext::WildSubdue
        };
        let seed = 1000 + self.session.battles_fought as u64 * 7919;
        let rider_mods = RiderMods::from_rider(&self.session.profile.rider);
        match Battle::new(&self.data, context, &player, &enemy, rider_mods, seed) {
            Ok(battle) => {
                self.mode = Mode::Battle(Box::new(BattleScreen::new(battle, self.session.pace)));
            }
            Err(err) => self
                .notifications
                .danger(format!("Battle setup failed: {}", err)),
        }
    }

    fn save_game(&mut self) {
        let save = SaveData::from_session(&self.session, &self.data.config.version);
        match save_to_slot_with_version(
            &self.data.config.game_name,
            &self.data.config.save_slot,
            &save,
            &self.data.config.version,
        ) {
            Ok(()) => {
                self.notifications.success("Saved");
                self.save_exists = true;
            }
            Err(err) => self.notifications.danger(format!("Save failed: {}", err)),
        }
    }

    fn load_game(&mut self) {
        let version = self.data.config.version.clone();
        let loaded: Result<SaveData, String> = load_from_slot_with_migration(
            &self.data.config.game_name,
            &self.data.config.save_slot,
            &version,
            |detected, value| migrate_save_value(detected, value, &version),
        );
        match loaded {
            Ok(save) => {
                self.session = save.session;
                self.notifications.success("Loaded");
            }
            Err(err) => self.notifications.warning(format!("Load failed: {}", err)),
        }
    }
}

fn wild(species: &str) -> UnitSpec {
    UnitSpec {
        species_id: species.to_owned(),
        name: format!("wild {}", species),
        side: Side::Enemy,
        creature_id: None,
        bond: 0.0,
        stance: Stance::Aggressive,
        grafts: Vec::new(),
    }
}

fn armed(species: &str, grafts: Vec<(&str, usize, &str)>) -> UnitSpec {
    UnitSpec {
        species_id: species.to_owned(),
        name: format!("war-unit {}", species),
        side: Side::Enemy,
        creature_id: None,
        bond: 0.0,
        stance: Stance::Aggressive,
        grafts: grafts
            .into_iter()
            .map(|(l, s, d)| (l.to_owned(), s, d.to_owned(), None))
            .collect(),
    }
}
