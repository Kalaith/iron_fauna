//! High-level game loop and state transitions.

use crate::combat::engine::Battle;
use crate::combat::unit::UnitSpec;
use crate::combat::{resolve, BattleContext, RiderMods, Side, Stance};
use crate::data::GameData;
use crate::model::duel::{self, PendingDuel};
use crate::state::{migrate_save_value, GameSession, SaveData};
use crate::ui::battle::{BattleScreen, BattleScreenResult};
use crate::ui::outfit::{OutfitAction, OutfitScreen};
use crate::ui::overworld::{OverworldResult, OverworldScreen};
use crate::ui::settlement::{sell_price, SettlementAction, SettlementScreen, SettlementView};
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
    Overworld(OverworldScreen),
    Settlement(SettlementScreen),
    Outfit(OutfitScreen),
    Battle(Box<BattleScreen>),
}

/// Where a sub-screen (battle, bench) hands control back to.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ReturnTo {
    Menu,
    Overworld,
    Settlement(String),
}

pub struct Game {
    data: GameData,
    session: GameSession,
    #[allow(dead_code)]
    assets: AssetManager,
    notifications: NotificationManager,
    mode: Mode,
    return_to: ReturnTo,
    pending_duel: Option<PendingDuel>,
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
            return_to: ReturnTo::Menu,
            pending_duel: None,
            save_exists,
        }
    }

    /// Seeds a named state for the headless screenshot harness.
    pub fn begin_capture_scene(&mut self, scene: &str) {
        match scene {
            "overworld" => {
                self.return_to = ReturnTo::Overworld;
                self.mode = Mode::Overworld(OverworldScreen::new(&self.session));
            }
            "battle" => {
                self.return_to = ReturnTo::Menu;
                self.start_dev_battle();
            }
            "outfit" => {
                self.mode = Mode::Outfit(OutfitScreen {
                    selected: self.session.profile.roster.party.first().copied(),
                    selected_slot: None,
                });
            }
            "settlement" => {
                let mut screen = SettlementScreen::new("fernhollow");
                screen.view = SettlementView::Ring;
                self.mode = Mode::Settlement(screen);
            }
            _ => self.mode = Mode::Menu,
        }
    }

    fn resume(&mut self) {
        self.mode = match &self.return_to {
            ReturnTo::Menu => Mode::Menu,
            ReturnTo::Overworld => Mode::Overworld(OverworldScreen::new(&self.session)),
            ReturnTo::Settlement(id) => Mode::Settlement(SettlementScreen::new(id)),
        };
    }

    pub fn update(&mut self, dt: f32) {
        self.notifications.update(dt);

        match &mut self.mode {
            Mode::Battle(screen) => match screen.update(&self.data, dt) {
                BattleScreenResult::Continue => {}
                BattleScreenResult::Finished => self.finish_battle(),
            },
            Mode::Overworld(screen) => match screen.update(&self.data, &mut self.session, dt) {
                OverworldResult::Continue => {}
                OverworldResult::BackToMenu => self.mode = Mode::Menu,
                OverworldResult::OpenSettlement => {
                    let settlement = self
                        .data
                        .world
                        .map(&self.session.location.map_id)
                        .and_then(|m| m.settlement.clone());
                    if let Some(id) = settlement {
                        self.mode = Mode::Settlement(SettlementScreen::new(&id));
                    }
                }
                OverworldResult::StartEncounter(pack) => {
                    self.return_to = ReturnTo::Overworld;
                    self.start_battle(BattleContext::WildSubdue, pack);
                }
            },
            _ => {}
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
        if let Some(pending) = self.pending_duel.take() {
            let won = matches!(
                screen.battle.outcome,
                Some(crate::combat::BattleOutcome::Victory(_))
            );
            let duelist = self
                .data
                .settlements
                .get(&pending.settlement_id)
                .and_then(|s| s.duelist(&pending.duelist_id))
                .cloned();
            if let Some(duelist) = duelist {
                for line in
                    duel::apply_duel_result(&mut self.session, &self.data, &pending, &duelist, won)
                {
                    self.notifications.info(line);
                }
            }
        }
        self.resume();
    }

    fn start_battle(&mut self, context: BattleContext, enemy: Vec<UnitSpec>) {
        let player = self.party_unit_specs();
        if player.is_empty() {
            self.notifications.warning("No creatures in the party");
            return;
        }
        let seed =
            1000 + self.session.battles_fought as u64 * 7919 + self.session.steps.wrapping_mul(31);
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

    pub fn draw(&mut self) {
        clear_background(dark::BACKGROUND);
        let virtual_ui = begin_virtual_ui_frame(ui::LOGICAL_WIDTH, ui::LOGICAL_HEIGHT);

        let mut actions = Vec::new();
        let mut outfit_actions = Vec::new();
        let mut settlement_actions = Vec::new();
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
            Mode::Overworld(screen) => screen.draw(&self.data, &self.session),
            Mode::Settlement(screen) => {
                settlement_actions = screen.draw(&self.data, &self.session, &virtual_ui);
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
        for action in settlement_actions {
            self.apply_settlement_action(action);
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
            UiAction::EnterWorld => {
                self.return_to = ReturnTo::Overworld;
                self.mode = Mode::Overworld(OverworldScreen::new(&self.session));
            }
            UiAction::StartDevBattle => self.start_dev_battle(),
            UiAction::OpenOutfit => {
                self.return_to = ReturnTo::Menu;
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
            OutfitAction::Back => self.resume(),
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

    fn apply_settlement_action(&mut self, action: SettlementAction) {
        let Mode::Settlement(screen) = &mut self.mode else {
            return;
        };
        let settlement_id = screen.settlement_id.clone();
        match action {
            SettlementAction::ShowHub => screen.view = SettlementView::Hub,
            SettlementAction::ShowShop => screen.view = SettlementView::Shop,
            SettlementAction::ShowRing => screen.view = SettlementView::Ring,
            SettlementAction::PickStake(duelist) => {
                screen.view = SettlementView::StakePick { duelist };
            }
            SettlementAction::Leave => {
                self.return_to = ReturnTo::Overworld;
                self.resume();
            }
            SettlementAction::OpenBench => {
                self.return_to = ReturnTo::Settlement(settlement_id);
                self.mode = Mode::Outfit(OutfitScreen {
                    selected: self.session.profile.roster.party.first().copied(),
                    selected_slot: None,
                });
            }
            SettlementAction::Buy(def_id) => {
                let price = self
                    .data
                    .settlements
                    .get(&settlement_id)
                    .and_then(|s| s.shop.iter().find(|e| e.graft == def_id))
                    .and_then(|e| e.price)
                    .or_else(|| self.data.graftware.get(&def_id).map(|d| d.value));
                let Some(price) = price else {
                    return;
                };
                if self.session.profile.inventory.scrip < price {
                    self.notifications.warning("Not enough scrip");
                    return;
                }
                self.session.profile.inventory.scrip -= price;
                self.session
                    .profile
                    .grant_graft(&def_id, crate::model::inventory::GraftCondition::Intact);
                let name = self
                    .data
                    .graftware
                    .get(&def_id)
                    .map(|d| d.name.clone())
                    .unwrap_or(def_id);
                self.notifications.success(format!("Bought {}", name));
            }
            SettlementAction::Sell(item_id) => {
                let equipped = self.session.profile.equipped_item_ids();
                if equipped.contains(&item_id) {
                    return;
                }
                let Some(item) = self.session.profile.inventory.item(item_id) else {
                    return;
                };
                let Some(def) = self.data.graftware.get(&item.def_id) else {
                    return;
                };
                let price = sell_price(def.value);
                let name = def.name.clone();
                self.session
                    .profile
                    .inventory
                    .items
                    .retain(|i| i.id != item_id);
                self.session.profile.inventory.scrip += price;
                self.notifications
                    .info(format!("Sold {} for {}", name, price));
            }
            SettlementAction::Challenge { duelist, stake } => {
                self.start_duel(&settlement_id, &duelist, stake);
            }
        }
    }

    fn start_duel(&mut self, settlement_id: &str, duelist_id: &str, stake: Option<u64>) {
        let Some(duelist) = self
            .data
            .settlements
            .get(settlement_id)
            .and_then(|s| s.duelist(duelist_id))
            .cloned()
        else {
            return;
        };
        if !duel::can_challenge(&self.session, settlement_id, &duelist) {
            self.notifications.warning("The ring won't allow it yet");
            return;
        }
        if !duelist.practice && stake.is_none() {
            return;
        }
        let enemy: Vec<UnitSpec> = duelist
            .party
            .iter()
            .map(|u| UnitSpec {
                species_id: u.species.clone(),
                name: u
                    .name
                    .clone()
                    .unwrap_or_else(|| format!("{}'s creature", duelist.name)),
                side: Side::Enemy,
                creature_id: None,
                bond: 0.0,
                stance: Stance::Aggressive,
                grafts: u
                    .grafts
                    .iter()
                    .map(|g| (g.limb.clone(), g.slot, g.graft.clone(), None))
                    .collect(),
            })
            .collect();
        self.pending_duel = Some(PendingDuel {
            settlement_id: settlement_id.to_owned(),
            duelist_id: duelist_id.to_owned(),
            my_stake: stake,
        });
        self.return_to = ReturnTo::Settlement(settlement_id.to_owned());
        self.start_battle(BattleContext::Duel, enemy);
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

    /// A deterministic sample encounter for engine testing from the menu.
    fn start_dev_battle(&mut self) {
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
        self.return_to = ReturnTo::Menu;
        self.start_battle(context, enemy);
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
