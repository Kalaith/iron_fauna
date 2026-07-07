//! Settlement hub: the bench, the supply post, and the duelling ring
//! (`game_design.md` §10).

use crate::data::settlement::SettlementDef;
use crate::data::GameData;
use crate::model::duel;
use crate::state::GameSession;
use crate::ui::{menu_button, LOGICAL_HEIGHT, LOGICAL_WIDTH};
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::{draw_ui_text_ex, RectExt, VirtualUi};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettlementAction {
    ShowHub,
    ShowShop,
    ShowRing,
    OpenBench,
    Leave,
    Buy(String),
    Sell(u64),
    /// Fund the region's watch — stewardship against relapse (§9.1).
    FundWatch,
    /// Open the stake picker for a staked duelist.
    PickStake(String),
    Challenge {
        duelist: String,
        stake: Option<u64>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettlementView {
    Hub,
    Shop,
    Ring,
    StakePick { duelist: String },
}

pub struct SettlementScreen {
    pub settlement_id: String,
    pub view: SettlementView,
}

impl SettlementScreen {
    pub fn new(settlement_id: &str) -> Self {
        Self {
            settlement_id: settlement_id.to_owned(),
            view: SettlementView::Hub,
        }
    }

    pub fn draw(
        &self,
        data: &GameData,
        session: &GameSession,
        ui: &VirtualUi,
    ) -> Vec<SettlementAction> {
        let mut actions = Vec::new();
        let mouse = ui.mouse_position();
        let Some(settlement) = data.settlements.get(&self.settlement_id) else {
            actions.push(SettlementAction::Leave);
            return actions;
        };

        if is_key_pressed(KeyCode::Escape) {
            actions.push(if self.view == SettlementView::Hub {
                SettlementAction::Leave
            } else {
                SettlementAction::ShowHub
            });
        }

        self.draw_header(settlement, session, mouse, &mut actions);
        match &self.view {
            SettlementView::Hub => self.draw_hub(data, settlement, session, mouse, &mut actions),
            SettlementView::Shop => self.draw_shop(data, settlement, session, mouse, &mut actions),
            SettlementView::Ring => self.draw_ring(data, settlement, session, mouse, &mut actions),
            SettlementView::StakePick { duelist } => {
                self.draw_stake_pick(data, settlement, session, duelist, mouse, &mut actions)
            }
        }
        actions
    }

    fn draw_header(
        &self,
        settlement: &SettlementDef,
        session: &GameSession,
        mouse: Vec2,
        actions: &mut Vec<SettlementAction>,
    ) {
        let rect = Rect::new(18.0, 14.0, LOGICAL_WIDTH - 36.0, 52.0);
        draw_surface(
            rect,
            &SurfaceStyle::new(Color::new(0.09, 0.09, 0.11, 0.97))
                .with_border(1.0, Color::new(0.45, 0.42, 0.35, 0.7)),
        );
        draw_ui_text_ex(
            &settlement.name.to_uppercase(),
            rect.x + 16.0,
            rect.y + 33.0,
            TextStyle::new(24.0, Color::new(0.88, 0.84, 0.74, 1.0)).params(),
        );
        draw_ui_text_ex(
            &format!(
                "scrip {}   ·   ring rank {}",
                session.profile.inventory.scrip,
                duel::current_rank(session, &settlement.id)
            ),
            rect.right() - 340.0,
            rect.y + 33.0,
            TextStyle::new(17.0, dark::TEXT).params(),
        );
        if menu_button(
            Rect::new(rect.right() - 120.0, rect.y + 8.0, 106.0, 36.0),
            "Leave",
            true,
            mouse,
        ) {
            actions.push(if self.view == SettlementView::Hub {
                SettlementAction::Leave
            } else {
                SettlementAction::ShowHub
            });
        }
    }

    fn draw_hub(
        &self,
        data: &GameData,
        settlement: &SettlementDef,
        session: &GameSession,
        mouse: Vec2,
        actions: &mut Vec<SettlementAction>,
    ) {
        draw_text_block(
            &settlement.description,
            84.0,
            110.0,
            700.0,
            80.0,
            18.0,
            6.0,
            dark::TEXT_DIM,
        );

        let options: [(&str, &str, SettlementAction); 3] = [
            (
                "Grafting Bench",
                "Outfit war-bodies, repair parts, swap the party",
                SettlementAction::OpenBench,
            ),
            (
                "Supply Post",
                "Buy and sell basic graftware",
                SettlementAction::ShowShop,
            ),
            (
                "Duelling Ring",
                "Practice bouts and staked wagers against local riders",
                SettlementAction::ShowRing,
            ),
        ];
        for (i, (label, desc, action)) in options.iter().enumerate() {
            let rect = Rect::new(84.0, 220.0 + i as f32 * 96.0, 520.0, 78.0);
            let hovered = rect.contains_point(mouse);
            draw_surface(
                rect,
                &SurfaceStyle::new(if hovered {
                    Color::new(0.15, 0.17, 0.20, 1.0)
                } else {
                    Color::new(0.11, 0.12, 0.15, 1.0)
                })
                .with_border(1.0, Color::new(0.45, 0.48, 0.55, 0.5))
                .with_left_accent(4.0, Color::new(0.60, 0.52, 0.38, 1.0)),
            );
            draw_ui_text_ex(
                label,
                rect.x + 18.0,
                rect.y + 32.0,
                TextStyle::new(20.0, dark::TEXT_BRIGHT).params(),
            );
            draw_ui_text_ex(
                desc,
                rect.x + 18.0,
                rect.y + 58.0,
                TextStyle::new(14.0, dark::TEXT_DIM).params(),
            );
            if hovered && is_mouse_button_released(MouseButton::Left) {
                actions.push(action.clone());
            }
        }

        // Stewardship: a reseeded, untended region can be watched (§9.1).
        let factory_id = data
            .world
            .region(&settlement.region)
            .map(|r| r.gestarium_id.clone());
        if let Some(factory_id) = factory_id {
            let state = session.world_state.factory(&factory_id);
            if state.verdict == Some(crate::model::worldstate::Verdict::Reseed)
                && !state.invested
                && !state.relapsed
            {
                let cost = data.balance.world.watch_cost;
                let rect = Rect::new(84.0, 220.0 + 3.0 * 96.0, 520.0, 78.0);
                let hovered = rect.contains_point(mouse);
                draw_surface(
                    rect,
                    &SurfaceStyle::new(if hovered {
                        Color::new(0.14, 0.18, 0.15, 1.0)
                    } else {
                        Color::new(0.10, 0.13, 0.11, 1.0)
                    })
                    .with_border(1.0, Color::new(0.40, 0.60, 0.45, 0.6))
                    .with_left_accent(4.0, Color::new(0.40, 0.65, 0.45, 1.0)),
                );
                draw_ui_text_ex(
                    &format!("Fund the Watch — {} scrip", cost),
                    rect.x + 18.0,
                    rect.y + 32.0,
                    TextStyle::new(20.0, dark::TEXT_BRIGHT).params(),
                );
                draw_ui_text_ex(
                    "Station eyes on the reseeded factory so the old temptation stays buried",
                    rect.x + 18.0,
                    rect.y + 58.0,
                    TextStyle::new(14.0, dark::TEXT_DIM).params(),
                );
                if hovered && is_mouse_button_released(MouseButton::Left) {
                    actions.push(SettlementAction::FundWatch);
                }
            }
        }
    }

    fn draw_shop(
        &self,
        data: &GameData,
        settlement: &SettlementDef,
        session: &GameSession,
        mouse: Vec2,
        actions: &mut Vec<SettlementAction>,
    ) {
        // Wares.
        let rect = Rect::new(18.0, 80.0, 600.0, LOGICAL_HEIGHT - 98.0);
        draw_surface_with_title(
            rect,
            Some("Supply Post — wares"),
            &panel_style(),
            TextStyle::new(16.0, dark::TEXT),
        );
        let content = rect.inset(14.0);
        let mut y = content.y + 40.0;
        for entry in &settlement.shop {
            let Some(def) = data.graftware.get(&entry.graft) else {
                continue;
            };
            let price = entry.price.unwrap_or(def.value);
            let row = Rect::new(content.x, y, content.w, 46.0);
            draw_surface(row, &SurfaceStyle::new(Color::new(0.10, 0.115, 0.15, 1.0)));
            draw_ui_text_ex(
                &format!(
                    "{} · {} {}",
                    def.name,
                    def.weight.display_name(),
                    def.kind.display_name()
                ),
                row.x + 10.0,
                row.y + 19.0,
                TextStyle::new(15.0, dark::TEXT).params(),
            );
            draw_ui_text_ex(
                &format!("draw {} · min pwr {}", def.power_draw, def.min_power),
                row.x + 10.0,
                row.y + 37.0,
                TextStyle::new(12.0, dark::TEXT_DIM).params(),
            );
            if menu_button(
                Rect::new(row.right() - 110.0, row.y + 8.0, 102.0, 30.0),
                &format!("Buy {}", price),
                session.profile.inventory.scrip >= price,
                mouse,
            ) {
                actions.push(SettlementAction::Buy(entry.graft.clone()));
            }
            y += 52.0;
        }

        // Sell list: unequipped, intact parts fetch 40% of value.
        let rect = Rect::new(636.0, 80.0, 626.0, LOGICAL_HEIGHT - 98.0);
        draw_surface_with_title(
            rect,
            Some("Sell — unequipped parts (40% value)"),
            &panel_style(),
            TextStyle::new(16.0, dark::TEXT),
        );
        let content = rect.inset(14.0);
        let mut y = content.y + 40.0;
        let equipped = session.profile.equipped_item_ids();
        for item in &session.profile.inventory.items {
            if equipped.contains(&item.id) || !item.is_usable() {
                continue;
            }
            let Some(def) = data.graftware.get(&item.def_id) else {
                continue;
            };
            if y > content.bottom() - 46.0 {
                break;
            }
            let row = Rect::new(content.x, y, content.w, 40.0);
            draw_surface(row, &SurfaceStyle::new(Color::new(0.10, 0.115, 0.15, 1.0)));
            draw_ui_text_ex(
                &def.name,
                row.x + 10.0,
                row.y + 25.0,
                TextStyle::new(15.0, dark::TEXT).params(),
            );
            if menu_button(
                Rect::new(row.right() - 110.0, row.y + 5.0, 102.0, 30.0),
                &format!("Sell {}", sell_price(def.value)),
                true,
                mouse,
            ) {
                actions.push(SettlementAction::Sell(item.id));
            }
            y += 46.0;
        }
    }

    fn draw_ring(
        &self,
        data: &GameData,
        settlement: &SettlementDef,
        session: &GameSession,
        mouse: Vec2,
        actions: &mut Vec<SettlementAction>,
    ) {
        let rect = Rect::new(18.0, 80.0, LOGICAL_WIDTH - 36.0, LOGICAL_HEIGHT - 98.0);
        draw_surface_with_title(
            rect,
            Some("Duelling Ring — crack the shell, take the stake, harm nothing"),
            &panel_style(),
            TextStyle::new(16.0, dark::TEXT),
        );
        let content = rect.inset(16.0);
        let mut y = content.y + 44.0;

        for duelist in &settlement.duelists {
            let row = Rect::new(content.x, y, content.w, 86.0);
            let allowed = duel::can_challenge(session, &settlement.id, duelist);
            draw_surface(
                row,
                &SurfaceStyle::new(Color::new(0.10, 0.115, 0.15, 1.0)).with_left_accent(
                    4.0,
                    if duelist.practice {
                        Color::new(0.40, 0.60, 0.45, 1.0)
                    } else {
                        Color::new(0.75, 0.55, 0.30, 1.0)
                    },
                ),
            );
            draw_ui_text_ex(
                &format!(
                    "{}   —   {}   ·   rank {}+   ·   {} scrip",
                    duelist.name,
                    if duelist.practice {
                        "practice"
                    } else {
                        "STAKED"
                    },
                    duelist.rank_req,
                    duelist.reward_scrip
                ),
                row.x + 14.0,
                row.y + 24.0,
                TextStyle::new(
                    16.0,
                    if allowed {
                        dark::TEXT_BRIGHT
                    } else {
                        dark::TEXT_DIM
                    },
                )
                .params(),
            );
            draw_text_block(
                &duelist.blurb,
                row.x + 14.0,
                row.y + 34.0,
                row.w - 260.0,
                40.0,
                13.0,
                4.0,
                dark::TEXT_DIM,
            );
            if let Some(stake) = &duelist.stake {
                let stake_name = data
                    .graftware
                    .get(stake)
                    .map(|d| d.name.as_str())
                    .unwrap_or(stake);
                draw_ui_text_ex(
                    &format!(
                        "stakes: {}  (your ante: value {}+)",
                        stake_name, duelist.min_stake_value
                    ),
                    row.x + 14.0,
                    row.bottom() - 10.0,
                    TextStyle::new(13.0, Color::new(0.8, 0.65, 0.4, 1.0)).params(),
                );
            }

            let can_fight = allowed
                && (duelist.practice
                    || !duel::eligible_stakes(session, data, duelist.min_stake_value).is_empty());
            let label = if !allowed {
                "Rank low"
            } else if duelist.practice {
                "Challenge"
            } else if can_fight {
                "Wager"
            } else {
                "No ante"
            };
            if menu_button(
                Rect::new(row.right() - 130.0, row.y + 24.0, 118.0, 38.0),
                label,
                can_fight,
                mouse,
            ) {
                if duelist.practice {
                    actions.push(SettlementAction::Challenge {
                        duelist: duelist.id.clone(),
                        stake: None,
                    });
                } else {
                    actions.push(SettlementAction::PickStake(duelist.id.clone()));
                }
            }
            y += 94.0;
        }
    }

    fn draw_stake_pick(
        &self,
        data: &GameData,
        settlement: &SettlementDef,
        session: &GameSession,
        duelist_id: &str,
        mouse: Vec2,
        actions: &mut Vec<SettlementAction>,
    ) {
        let Some(duelist) = settlement.duelist(duelist_id) else {
            actions.push(SettlementAction::ShowRing);
            return;
        };
        let rect = Rect::new(240.0, 110.0, 800.0, 500.0);
        draw_surface_with_title(
            rect,
            Some(&format!(
                "Ante up — {} wants value {}+ on the line",
                duelist.name, duelist.min_stake_value
            )),
            &panel_style(),
            TextStyle::new(16.0, dark::TEXT),
        );
        let content = rect.inset(16.0);
        let mut y = content.y + 44.0;
        draw_ui_text_ex(
            "Equipped graftware is protected — you can only stake what you can afford to lose.",
            content.x,
            y,
            TextStyle::new(14.0, dark::TEXT_DIM).params(),
        );
        y += 26.0;

        for item_id in duel::eligible_stakes(session, data, duelist.min_stake_value) {
            let Some(item) = session.profile.inventory.item(item_id) else {
                continue;
            };
            let Some(def) = data.graftware.get(&item.def_id) else {
                continue;
            };
            if y > content.bottom() - 46.0 {
                break;
            }
            let row = Rect::new(content.x, y, content.w, 40.0);
            draw_surface(row, &SurfaceStyle::new(Color::new(0.10, 0.115, 0.15, 1.0)));
            draw_ui_text_ex(
                &format!("{}  ·  value {}", def.name, def.value),
                row.x + 10.0,
                row.y + 25.0,
                TextStyle::new(15.0, dark::TEXT).params(),
            );
            if menu_button(
                Rect::new(row.right() - 130.0, row.y + 5.0, 122.0, 30.0),
                "Stake it",
                true,
                mouse,
            ) {
                actions.push(SettlementAction::Challenge {
                    duelist: duelist.id.clone(),
                    stake: Some(item_id),
                });
            }
            y += 46.0;
        }
        if menu_button(
            Rect::new(content.x, content.bottom() - 40.0, 120.0, 34.0),
            "Back",
            true,
            mouse,
        ) {
            actions.push(SettlementAction::ShowRing);
        }
    }
}

pub fn sell_price(value: i64) -> i64 {
    (value * 2 / 5).max(1)
}

fn panel_style() -> SurfaceStyle {
    SurfaceStyle::new(Color::new(0.08, 0.085, 0.105, 0.97))
        .with_border(1.0, Color::new(0.38, 0.45, 0.58, 0.65))
        .with_header(36.0, Color::new(0.105, 0.12, 0.15, 1.0))
        .with_header_divider(1.0, Color::new(0.38, 0.45, 0.58, 0.4))
}
