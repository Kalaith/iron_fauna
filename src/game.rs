//! High-level game loop and state transitions.

mod actions;
mod capture;

use crate::audio::Audio;
use crate::combat::engine::Battle;
use crate::combat::unit::UnitSpec;
use crate::combat::{resolve, BattleContext, RiderMods, Side, Stance};
use crate::data::world::DoorTarget;
use crate::data::GameData;
use crate::model::duel::{self, PendingDuel};
use crate::model::worldstate::Verdict;
use crate::state::{migrate_save_value, GameSession, SaveData};
use crate::ui::battle::{BattleScreen, BattleScreenResult};
use crate::ui::bestiary::BestiaryScreen;
use crate::ui::codex::CodexScreen;
use crate::ui::ledger::LedgerScreen;
use crate::ui::outfit::OutfitScreen;
use crate::ui::overworld::{OverworldResult, OverworldScreen};
use crate::ui::settlement::{SettlementScreen, SettlementView};
use crate::ui::verdict::{FactoryScreenKind, VerdictScreen};
use crate::ui::{self, MenuContext};
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
    Overworld(Box<OverworldScreen>),
    Settlement(SettlementScreen),
    Outfit(OutfitScreen),
    Battle(Box<BattleScreen>),
    FactoryHeart(VerdictScreen),
    Ledger(LedgerScreen),
    Bestiary(BestiaryScreen),
    Codex(Box<CodexScreen>),
}

/// Where a sub-screen (battle, bench) hands control back to.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ReturnTo {
    Menu,
    Overworld,
    Settlement(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BossKind {
    /// First assault on a live factory heart.
    Heart,
    /// Confronting a relapsed region's new keeper (§9.1).
    Relapse,
}

pub struct Game {
    data: GameData,
    session: GameSession,
    assets: AssetManager,
    notifications: NotificationManager,
    audio: Audio,
    mode: Mode,
    return_to: ReturnTo,
    pending_duel: Option<PendingDuel>,
    /// Set while a factory-heart or relapse-confrontation fight is underway.
    pending_boss: Option<(String, BossKind)>,
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
        // Populate the sprite UI-skin cache from the just-loaded textures.
        ui::skin::init(&assets);

        let session = GameSession::new_game(&data);
        let save_exists = slot_exists(&data.config.game_name, &data.config.save_slot);
        let audio = Audio::load().await;

        Self {
            data,
            session,
            assets,
            notifications: NotificationManager::new(),
            audio,
            mode: Mode::Menu,
            return_to: ReturnTo::Menu,
            pending_duel: None,
            pending_boss: None,
            save_exists,
        }
    }

    fn resume(&mut self) {
        self.mode = match &self.return_to {
            ReturnTo::Menu => Mode::Menu,
            ReturnTo::Overworld => Mode::Overworld(Box::new(OverworldScreen::new(&self.session))),
            ReturnTo::Settlement(id) => Mode::Settlement(SettlementScreen::new(id)),
        };
    }

    pub fn update(&mut self, dt: f32) {
        self.notifications.update(dt);

        match &mut self.mode {
            Mode::Battle(screen) => match screen.update(&self.data, &self.audio, dt) {
                BattleScreenResult::Continue => {}
                BattleScreenResult::Finished => self.finish_battle(),
            },
            Mode::Overworld(screen) => match screen.update(&self.data, &mut self.session, dt) {
                OverworldResult::Continue => {}
                OverworldResult::BackToMenu => self.mode = Mode::Menu,
                OverworldResult::OpenSettlement(target) => self.enter_building(target),
                OverworldResult::StartEncounter(pack) => {
                    self.return_to = ReturnTo::Overworld;
                    self.start_battle(BattleContext::WildSubdue, pack);
                }
                OverworldResult::HeartInteract(factory_id) => self.heart_interact(&factory_id),
                OverworldResult::OpenCodex => {
                    self.mode = Mode::Codex(Box::new(CodexScreen::new(&self.session, None)));
                }
            },
            _ => {}
        }
    }

    /// Steps through a settlement door into a specific facility. Bench opens
    /// the grafting screen directly; the rest open the town screen on that view.
    fn enter_building(&mut self, target: DoorTarget) {
        let Some(id) = self
            .data
            .world
            .map(&self.session.location.map_id)
            .and_then(|m| m.settlement.clone())
        else {
            return;
        };
        match target {
            DoorTarget::Bench => {
                self.return_to = ReturnTo::Overworld;
                self.mode = Mode::Outfit(OutfitScreen {
                    selected: self.session.profile.roster.party.first().copied(),
                    selected_slot: None,
                });
            }
            DoorTarget::Shop | DoorTarget::Ring | DoorTarget::Hub => {
                let mut screen = SettlementScreen::new(&id);
                screen.view = match target {
                    DoorTarget::Shop => SettlementView::Shop,
                    DoorTarget::Ring => SettlementView::Ring,
                    _ => SettlementView::Hub,
                };
                self.mode = Mode::Settlement(screen);
            }
        }
    }

    fn guard_specs(&self, factory_id: &str, relapse: bool) -> Vec<UnitSpec> {
        let Some(factory) = self.data.factories.get(factory_id) else {
            return Vec::new();
        };
        factory
            .heart_guard
            .iter()
            .map(|u| UnitSpec {
                species_id: u.species.clone(),
                name: if relapse {
                    format!("Relapsed {}", u.name.clone().unwrap_or_default())
                } else {
                    u.name.clone().unwrap_or_else(|| factory.name.clone())
                },
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
            .collect()
    }

    fn heart_interact(&mut self, factory_id: &str) {
        let state = self.session.world_state.factory(factory_id);
        if !state.heart_defeated {
            // The heart defends itself: the authored guardian fight.
            let enemy = self.guard_specs(factory_id, false);
            if enemy.is_empty() {
                return;
            }
            self.pending_boss = Some((factory_id.to_owned(), BossKind::Heart));
            self.return_to = ReturnTo::Overworld;
            self.start_battle(BattleContext::FactoryDismantle, enemy);
        } else if state.relapsed {
            // The place you saved, militarized — confront what you made
            // possible (`game_design.md` §9.1).
            let enemy = self.guard_specs(factory_id, true);
            if enemy.is_empty() {
                return;
            }
            self.pending_boss = Some((factory_id.to_owned(), BossKind::Relapse));
            self.return_to = ReturnTo::Overworld;
            self.start_battle(BattleContext::FactoryDismantle, enemy);
        } else if state.verdict == Some(Verdict::Bind) {
            self.mode = Mode::FactoryHeart(VerdictScreen {
                factory_id: factory_id.to_owned(),
                kind: FactoryScreenKind::Grow,
            });
        } else if state.verdict.is_none() {
            self.mode = Mode::FactoryHeart(VerdictScreen {
                factory_id: factory_id.to_owned(),
                kind: FactoryScreenKind::Verdict,
            });
        } else {
            self.notifications.info("The vats are still. It is done.");
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
        let won = matches!(
            screen.battle.outcome,
            Some(crate::combat::BattleOutcome::Victory(_))
        );
        // Subduing wild creatures advances any open bounty quest.
        if screen.battle.context == BattleContext::WildSubdue {
            if let Some(crate::combat::BattleOutcome::Victory(rewards)) = &screen.battle.outcome {
                let subdued = rewards.captured_species.len() as u32;
                if subdued > 0 {
                    for line in
                        crate::model::quest::advance_subdue(&mut self.session, &self.data, subdued)
                    {
                        self.notifications.success(line);
                    }
                }
            }
        }
        if let Some(pending) = self.pending_duel.take() {
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
        if let Some((factory_id, kind)) = self.pending_boss.take() {
            if won {
                match kind {
                    BossKind::Heart => {
                        self.session
                            .world_state
                            .factory_mut(&factory_id)
                            .heart_defeated = true;
                        let fname = self
                            .data
                            .factories
                            .get(&factory_id)
                            .map(|f| f.name.clone())
                            .unwrap_or_else(|| factory_id.clone());
                        crate::model::journal::record(
                            &mut self.session,
                            format!("Cracked the heart of {} and freed what it held.", fname),
                        );
                        // Defeating a Gestarium marks the rider (§3).
                        if let Some(factory) = self.data.factories.get(&factory_id) {
                            if self.session.profile.rider.grant(factory.rider_upgrade) {
                                self.notifications.success(format!(
                                    "Rider upgrade: {} — {}",
                                    factory.rider_upgrade.display_name(),
                                    factory.rider_upgrade.description()
                                ));
                            }
                        }
                        self.mode = Mode::FactoryHeart(VerdictScreen {
                            factory_id,
                            kind: FactoryScreenKind::Verdict,
                        });
                        return;
                    }
                    BossKind::Relapse => {
                        let state = self.session.world_state.factory_mut(&factory_id);
                        state.relapsed = false;
                        state.relapse_risk = 0.0;
                        crate::model::journal::record(
                            &mut self.session,
                            "Put down the keeper of a region I brought back — the ground I made possible.",
                        );
                        self.notifications.info(
                            "The new keeper yields. The region breathes again — watch it closer this time.",
                        );
                    }
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
        let mut verdict_actions = Vec::new();
        let mut codex_actions = Vec::new();
        let mut close_overlay = false;
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
            Mode::Overworld(screen) => screen.draw(&self.data, &self.session, &self.assets),
            Mode::Settlement(screen) => {
                settlement_actions = screen.draw(&self.data, &self.session, &virtual_ui);
            }
            Mode::Outfit(screen) => {
                outfit_actions = screen.draw(&self.data, &self.session, &virtual_ui);
            }
            Mode::Battle(screen) => screen.draw(&self.data),
            Mode::FactoryHeart(screen) => {
                verdict_actions = screen.draw(&self.data, &self.session, &virtual_ui);
            }
            Mode::Ledger(screen) => {
                if screen.draw(&self.data, &self.session, &virtual_ui) {
                    close_overlay = true;
                }
            }
            Mode::Bestiary(screen) => {
                if screen.draw(&self.data, &self.session, &virtual_ui) {
                    close_overlay = true;
                }
            }
            Mode::Codex(screen) => {
                codex_actions = screen.draw(&self.data, &self.session, &virtual_ui);
            }
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
        for action in verdict_actions {
            self.apply_verdict_action(action);
        }
        for action in codex_actions {
            self.apply_codex_action(action);
        }
        if close_overlay {
            self.mode = Mode::Menu;
        }

        self.notifications
            .draw_with_config(&NotificationRenderConfig {
                anchor: NotificationAnchor::BottomRight,
                ..Default::default()
            });
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
