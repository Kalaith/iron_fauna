//! Action dispatch: the `apply_*` handlers that turn each screen's returned
//! intents into game-state changes. Split out of `game.rs` to keep the loop
//! file focused on state transitions.

use super::{Game, Mode, ReturnTo};
use crate::audio::Sfx;
use crate::model::worldstate::Verdict;
use crate::state::GameSession;
use crate::ui::codex::{CodexAction, CodexScreen};
use crate::ui::ledger::LedgerScreen;
use crate::ui::outfit::{OutfitAction, OutfitScreen};
use crate::ui::overworld::OverworldScreen;
use crate::ui::settings::{SettingsAction, SettingsScreen};
use crate::ui::settlement::{sell_price, SettlementAction, SettlementScreen, SettlementView};
use crate::ui::verdict::VerdictAction;
use crate::ui::UiAction;

impl Game {
    pub(super) fn apply_action(&mut self, action: UiAction) {
        self.audio.play(Sfx::Select);
        match action {
            UiAction::NewGame => {
                self.session = GameSession::new_game(&self.data);
                self.notifications.info("A new rider takes the road");
            }
            UiAction::EnterWorld => {
                self.return_to = ReturnTo::Overworld;
                self.mode = Mode::Overworld(Box::new(OverworldScreen::new(&self.session)));
            }
            UiAction::StartDevBattle => self.start_dev_battle(),
            UiAction::Load => self.load_game(),
            UiAction::OpenSettings => self.mode = Mode::Settings(SettingsScreen),
            UiAction::ExitGame => self.quit_game(),
        }
    }

    pub(super) fn apply_settings_action(&mut self, action: SettingsAction) {
        self.audio.play(Sfx::Select);
        match action {
            SettingsAction::Back => self.mode = Mode::Menu,
            SettingsAction::TogglePace => {
                self.session.pace = self.session.pace.toggled();
                self.notifications
                    .info(format!("Pace: {}", self.session.pace.display_name()));
            }
        }
    }

    /// Quits the game. On the web the browser owns the tab, so this is a no-op
    /// there and the Exit button simply does nothing.
    pub(super) fn quit_game(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        std::process::exit(0);
    }

    pub(super) fn apply_outfit_action(&mut self, action: OutfitAction) {
        self.audio.play(Sfx::Select);
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

    pub(super) fn apply_settlement_action(&mut self, action: SettlementAction) {
        self.audio.play(Sfx::Select);
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
            SettlementAction::OpenCodex => {
                self.mode = Mode::Codex(Box::new(CodexScreen::new(
                    &self.session,
                    Some(settlement_id),
                )));
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
            SettlementAction::FundWatch => {
                let cost = self.data.balance.world.watch_cost;
                let factory_id = self
                    .data
                    .settlements
                    .get(&settlement_id)
                    .and_then(|s| self.data.world.region(&s.region))
                    .map(|r| r.gestarium_id.clone());
                let Some(factory_id) = factory_id else {
                    return;
                };
                if self.session.profile.inventory.scrip < cost {
                    self.notifications.warning("Not enough scrip");
                    return;
                }
                self.session.profile.inventory.scrip -= cost;
                self.session.world_state.factory_mut(&factory_id).invested = true;
                self.notifications
                    .success("The watch is funded. Someone will be looking.");
            }
        }
    }

    pub(super) fn apply_codex_action(&mut self, action: CodexAction) {
        let Mode::Codex(screen) = &mut self.mode else {
            return;
        };
        match action {
            CodexAction::Close => {
                // Return to wherever the codex was opened from.
                match screen.return_settlement.take() {
                    Some(id) => self.mode = Mode::Settlement(SettlementScreen::new(&id)),
                    None => {
                        self.return_to = ReturnTo::Overworld;
                        self.resume();
                    }
                }
            }
            CodexAction::Show(tab) => screen.tab = tab,
            CodexAction::Select(id) => screen.selected = Some(id),
            CodexAction::MoveUp(id) => {
                self.session.profile.roster.move_in_party(id, -1);
            }
            CodexAction::MoveDown(id) => {
                self.session.profile.roster.move_in_party(id, 1);
            }
            CodexAction::ToParty(id) => {
                if !self.session.profile.roster.add_to_party(&self.data, id) {
                    self.notifications.warning("No room in the party");
                }
            }
            CodexAction::ToStorage(id) => {
                if self.session.profile.roster.party.len() <= 1 {
                    self.notifications
                        .warning("The road is no place to walk alone");
                } else {
                    self.session.profile.roster.remove_from_party(id);
                }
            }
            CodexAction::OpenBench => {
                // Only offered from a settlement; return there afterwards.
                if let Some(id) = screen.return_settlement.take() {
                    self.return_to = ReturnTo::Settlement(id);
                    self.mode = Mode::Outfit(OutfitScreen {
                        selected: self.session.profile.roster.party.first().copied(),
                        selected_slot: None,
                    });
                }
            }
            CodexAction::FieldRepair => self.field_repair(),
            CodexAction::Save => self.save_game(),
            CodexAction::Load => self.load_game(),
            CodexAction::ExitGame => self.quit_game(),
        }
    }

    /// Spend one repair kit to mend the first damaged graft — the road's minor
    /// maintenance (`game_design.md` §4.4).
    fn field_repair(&mut self) {
        let inv = &mut self.session.profile.inventory;
        let Some(item_id) = inv.items.iter().find(|i| !i.is_usable()).map(|i| i.id) else {
            self.notifications.info("Nothing needs mending.");
            return;
        };
        if !inv.take_consumable("repair_kit") {
            self.notifications.warning("No repair kits left");
            return;
        }
        if let Some(item) = inv.item_mut(item_id) {
            item.condition = crate::model::inventory::GraftCondition::Intact;
        }
        self.notifications.success("Field-patched a graft");
    }

    pub(super) fn apply_verdict_action(&mut self, action: VerdictAction) {
        self.audio.play(Sfx::Select);
        let Mode::FactoryHeart(screen) = &self.mode else {
            return;
        };
        let factory_id = screen.factory_id.clone();
        match action {
            VerdictAction::Close => {
                self.return_to = ReturnTo::Overworld;
                self.resume();
            }
            VerdictAction::Choose(verdict) => {
                self.session.world_state.factory_mut(&factory_id).verdict = Some(verdict);
                let fname = self
                    .data
                    .factories
                    .get(&factory_id)
                    .map(|f| f.name.clone())
                    .unwrap_or_else(|| factory_id.clone());
                crate::model::journal::record(
                    &mut self.session,
                    format!("Passed {} on {}.", verdict.display_name(), fname),
                );
                let line = match verdict {
                    Verdict::Purge => {
                        "The wombs burn. The region is safe now — and it will never be alive again."
                    }
                    Verdict::Reseed => {
                        "The vats hum a softer note. Green will come back here. Watch it closely."
                    }
                    Verdict::Bind => {
                        "The machine acknowledges a new keeper. It does not ask what you'll grow."
                    }
                };
                self.notifications.info(line);
                self.return_to = ReturnTo::Overworld;
                // The last verdict in the world opens the closing reflection.
                if self.session.world_state.all_judged(&self.data) {
                    self.mode = Mode::Ledger(LedgerScreen);
                } else {
                    self.resume();
                }
            }
            VerdictAction::GrowCore(species_id) => {
                let Some(factory) = self.data.factories.get(&factory_id).cloned() else {
                    return;
                };
                if self.session.profile.inventory.scrip < factory.grow_cost {
                    self.notifications.warning("Not enough scrip");
                    return;
                }
                self.session.profile.inventory.scrip -= factory.grow_cost;
                self.session.profile.spawn_creature(
                    &self.data,
                    &species_id,
                    crate::model::creature::CreatureOrigin::Grown {
                        factory_id: factory_id.clone(),
                    },
                );
                let name = self
                    .data
                    .species
                    .get(&species_id)
                    .map(|s| s.name.as_str())
                    .unwrap_or(&species_id);
                self.notifications
                    .success(format!("A {} opens its eyes in the vat", name));
            }
        }
    }
}
