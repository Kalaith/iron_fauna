//! Camp screen: party management and graftware outfitting, with the live
//! Power Capacity readout from `creature.md` §5. Becomes the settlement
//! outfitting bench once the settlement layer lands.

use crate::data::GameData;
use crate::state::GameSession;
use crate::ui::{element_color, menu_button, LOGICAL_HEIGHT, LOGICAL_WIDTH};
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::{draw_ui_text_ex, RectExt, VirtualUi};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutfitAction {
    SelectCreature(u64),
    SelectSlot {
        limb_id: String,
        slot: usize,
    },
    Unequip {
        creature: u64,
        limb_id: String,
        slot: usize,
    },
    Equip {
        creature: u64,
        limb_id: String,
        slot: usize,
        item: u64,
    },
    Repair(u64),
    ToParty(u64),
    ToStorage(u64),
    Back,
}

#[derive(Debug, Default)]
pub struct OutfitScreen {
    pub selected: Option<u64>,
    pub selected_slot: Option<(String, usize)>,
}

impl OutfitScreen {
    pub fn draw(
        &self,
        data: &GameData,
        session: &GameSession,
        ui: &VirtualUi,
    ) -> Vec<OutfitAction> {
        let mut actions = Vec::new();
        let mouse = ui.mouse_position();

        if is_key_pressed(KeyCode::Escape) {
            actions.push(OutfitAction::Back);
        }

        self.draw_header(session, mouse, &mut actions);
        self.draw_roster(data, session, mouse, &mut actions);
        self.draw_creature_panel(data, session, mouse, &mut actions);
        self.draw_item_panel(data, session, mouse, &mut actions);
        actions
    }

    fn draw_header(&self, session: &GameSession, mouse: Vec2, actions: &mut Vec<OutfitAction>) {
        let rect = Rect::new(18.0, 14.0, LOGICAL_WIDTH - 36.0, 52.0);
        draw_surface(
            rect,
            &SurfaceStyle::new(Color::new(0.08, 0.09, 0.12, 0.96))
                .with_border(1.0, Color::new(0.38, 0.45, 0.58, 0.65)),
        );
        draw_ui_text_ex(
            "CAMP — party & grafting",
            rect.x + 16.0,
            rect.y + 33.0,
            TextStyle::new(24.0, dark::TEXT_BRIGHT).params(),
        );
        draw_ui_text_ex(
            &format!("scrip {}", session.profile.inventory.scrip),
            rect.right() - 300.0,
            rect.y + 33.0,
            TextStyle::new(18.0, dark::TEXT).params(),
        );
        if menu_button(
            Rect::new(rect.right() - 130.0, rect.y + 8.0, 116.0, 36.0),
            "Back [Esc]",
            true,
            mouse,
        ) {
            actions.push(OutfitAction::Back);
        }
    }

    fn draw_roster(
        &self,
        data: &GameData,
        session: &GameSession,
        mouse: Vec2,
        actions: &mut Vec<OutfitAction>,
    ) {
        let roster = &session.profile.roster;
        let rect = Rect::new(18.0, 80.0, 320.0, LOGICAL_HEIGHT - 98.0);
        let style = panel_style();
        draw_surface_with_title(
            rect,
            Some(&format!(
                "Roster — party {}/{}",
                roster.slots_used(data),
                data.balance.party_slot_budget
            )),
            &style,
            TextStyle::new(16.0, dark::TEXT),
        );
        let content = rect.inset(12.0);
        let mut y = content.y + 40.0;

        let slots_free = roster.slots_free(data);
        for creature in roster.party_members() {
            y = self.roster_row(data, mouse, actions, content, y, creature, true, slots_free);
        }
        y += 10.0;
        draw_ui_text_ex(
            "— settlement storage —",
            content.x + 4.0,
            y,
            TextStyle::new(14.0, dark::TEXT_DIM).params(),
        );
        y += 14.0;
        let mut any_stored = false;
        let stored: Vec<_> = roster.stored().collect();
        for creature in stored {
            any_stored = true;
            y = self.roster_row(
                data, mouse, actions, content, y, creature, false, slots_free,
            );
            if y > content.bottom() - 40.0 {
                break;
            }
        }
        if !any_stored {
            draw_ui_text_ex(
                "(empty)",
                content.x + 4.0,
                y + 16.0,
                TextStyle::new(14.0, dark::TEXT_DIM).params(),
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn roster_row(
        &self,
        data: &GameData,
        mouse: Vec2,
        actions: &mut Vec<OutfitAction>,
        content: Rect,
        y: f32,
        creature: &crate::model::creature::CreatureInstance,
        in_party: bool,
        slots_free: u32,
    ) -> f32 {
        let species = creature.species(data);
        let row = Rect::new(content.x, y, content.w, 46.0);
        let selected = self.selected == Some(creature.id);
        let hovered = row.contains_point(mouse);
        let fill = if selected {
            Color::new(0.18, 0.24, 0.32, 1.0)
        } else if hovered {
            Color::new(0.14, 0.17, 0.22, 1.0)
        } else {
            Color::new(0.10, 0.115, 0.15, 1.0)
        };
        draw_surface(row, &SurfaceStyle::new(fill));
        draw_circle(
            row.x + 16.0,
            row.y + 23.0,
            8.0,
            element_color(species.element),
        );
        draw_ui_text_ex(
            &format!(
                "{} ({})",
                creature.display_name(data),
                species.size.slot_cost()
            ),
            row.x + 32.0,
            row.y + 20.0,
            TextStyle::new(16.0, dark::TEXT).params(),
        );
        draw_ui_text_ex(
            &format!(
                "{} · {} mounts · bond {:.1}",
                species.size.display_name(),
                species.mount_count(),
                creature.bond
            ),
            row.x + 32.0,
            row.y + 38.0,
            TextStyle::new(12.0, dark::TEXT_DIM).params(),
        );
        if hovered && is_mouse_button_released(MouseButton::Left) {
            actions.push(OutfitAction::SelectCreature(creature.id));
        }
        let btn = Rect::new(row.right() - 62.0, row.y + 8.0, 56.0, 30.0);
        if in_party {
            if menu_button(btn, "Store", true, mouse) {
                actions.push(OutfitAction::ToStorage(creature.id));
            }
        } else {
            let fits = species.size.slot_cost() <= slots_free;
            if menu_button(btn, "Take", fits, mouse) {
                actions.push(OutfitAction::ToParty(creature.id));
            }
        }
        y + 52.0
    }

    fn draw_creature_panel(
        &self,
        data: &GameData,
        session: &GameSession,
        mouse: Vec2,
        actions: &mut Vec<OutfitAction>,
    ) {
        let rect = Rect::new(350.0, 80.0, 470.0, LOGICAL_HEIGHT - 98.0);
        let style = panel_style();
        let Some(creature) = self
            .selected
            .and_then(|id| session.profile.roster.creature(id))
        else {
            draw_surface_with_title(
                rect,
                Some("War-Body"),
                &style,
                TextStyle::new(16.0, dark::TEXT),
            );
            draw_ui_text_ex(
                "Select a creature to outfit.",
                rect.x + 20.0,
                rect.y + 70.0,
                TextStyle::new(16.0, dark::TEXT_DIM).params(),
            );
            return;
        };
        let species = creature.species(data);
        draw_surface_with_title(
            rect,
            Some(&format!("War-Body — {}", creature.display_name(data))),
            &style,
            TextStyle::new(16.0, dark::TEXT),
        );
        let content = rect.inset(16.0);
        let mut y = content.y + 42.0;

        draw_ui_text_ex(
            &format!(
                "{} · {} · {} · {}",
                species.element.display_name(),
                species.size.display_name(),
                species.temperament.display_name(),
                species.archetype.display_name(),
            ),
            content.x,
            y,
            TextStyle::new(15.0, dark::TEXT).params(),
        );
        y += 22.0;
        draw_ui_text_ex(
            &format!("Power {} · Speed {}", species.power, species.speed),
            content.x,
            y,
            TextStyle::new(15.0, dark::TEXT_DIM).params(),
        );
        y += 26.0;

        // Live Power Capacity readout (`creature.md` §5).
        let capacity = species.derived(&data.balance).power_capacity;
        let draw_total = creature.total_power_draw(data, &session.profile.inventory);
        let over = creature.overdraw(data, &session.profile.inventory);
        let frac = (draw_total / capacity).min(1.0);
        let bar = Rect::new(content.x, y, content.w, 20.0);
        draw_rectangle(
            bar.x,
            bar.y,
            bar.w,
            bar.h,
            Color::new(0.10, 0.11, 0.14, 1.0),
        );
        let color = if over > 0.0 {
            Color::new(0.85, 0.35, 0.25, 1.0)
        } else {
            Color::new(0.4, 0.65, 0.45, 1.0)
        };
        draw_rectangle(bar.x, bar.y, bar.w * frac, bar.h, color);
        draw_ui_text_ex(
            &format!(
                "Power Draw {:.0} / {:.0}{}",
                draw_total,
                capacity,
                if over > 0.0 {
                    format!("  OVERDRAWN +{:.0} — strain will climb", over)
                } else {
                    String::new()
                }
            ),
            bar.x + 8.0,
            bar.y + 15.0,
            TextStyle::new(13.0, dark::TEXT_BRIGHT).params(),
        );
        y += 34.0;

        // Limbs and mount slots.
        for limb in &species.limbs {
            draw_ui_text_ex(
                &format!("{}  ({})", limb.name, limb.region.display_name()),
                content.x,
                y + 14.0,
                TextStyle::new(14.0, dark::TEXT).params(),
            );
            let mut x = content.x + 190.0;
            if limb.mounts.is_empty() {
                draw_ui_text_ex(
                    "no mounts",
                    x,
                    y + 14.0,
                    TextStyle::new(13.0, dark::TEXT_DIM).params(),
                );
            }
            for (slot, class) in limb.mounts.iter().enumerate() {
                let btn = Rect::new(x, y - 4.0, 122.0, 26.0);
                let assigned = creature.assignment_at(&limb.id, slot);
                let is_sel = self.selected_slot.as_deref_pair() == Some((limb.id.as_str(), slot));
                let label = match assigned {
                    Some(a) => session
                        .profile
                        .inventory
                        .item(a.item_id)
                        .and_then(|i| data.graftware.get(&i.def_id))
                        .map(|d| d.name.clone())
                        .unwrap_or_else(|| "?".to_owned()),
                    None => format!("[{}]", class.display_name()),
                };
                let fill = if is_sel {
                    Color::new(0.25, 0.35, 0.30, 1.0)
                } else if assigned.is_some() {
                    Color::new(0.16, 0.19, 0.24, 1.0)
                } else {
                    Color::new(0.11, 0.13, 0.16, 1.0)
                };
                let hovered = btn.contains_point(mouse);
                draw_surface(
                    btn,
                    &SurfaceStyle::new(if hovered {
                        Color::new(fill.r + 0.05, fill.g + 0.05, fill.b + 0.05, 1.0)
                    } else {
                        fill
                    })
                    .with_border(1.0, Color::new(0.4, 0.45, 0.55, 0.5)),
                );
                draw_text_centered_in_box_ex(
                    &label,
                    btn.x + 4.0,
                    btn.y,
                    btn.w - 8.0,
                    btn.h,
                    TextStyle::new(12.0, dark::TEXT),
                );
                if hovered && is_mouse_button_released(MouseButton::Left) {
                    match assigned {
                        Some(_) => actions.push(OutfitAction::Unequip {
                            creature: creature.id,
                            limb_id: limb.id.clone(),
                            slot,
                        }),
                        None => actions.push(OutfitAction::SelectSlot {
                            limb_id: limb.id.clone(),
                            slot,
                        }),
                    }
                }
                x += 130.0;
            }
            y += 34.0;
        }

        y += 8.0;
        draw_ui_text_ex(
            "click an empty mount, then a part on the right to graft it",
            content.x,
            y + 8.0,
            TextStyle::new(13.0, dark::TEXT_DIM).params(),
        );

        // Rider progression summary (one upgrade per Gestarium).
        let mut ry = content.bottom() - 150.0;
        draw_ui_text_ex(
            &format!("Rider {} — Gestarium marks", session.profile.rider.name),
            content.x,
            ry,
            TextStyle::new(15.0, dark::TEXT_BRIGHT).params(),
        );
        ry += 20.0;
        for upgrade in crate::model::rider::RiderUpgrade::ALL {
            let earned = session.profile.rider.has(upgrade);
            draw_ui_text_ex(
                &format!(
                    "{} {} — {}",
                    if earned { "◆" } else { "◇" },
                    upgrade.display_name(),
                    upgrade.description()
                ),
                content.x,
                ry,
                TextStyle::new(12.0, if earned { dark::TEXT } else { dark::TEXT_DIM }).params(),
            );
            ry += 18.0;
        }
    }

    fn draw_item_panel(
        &self,
        data: &GameData,
        session: &GameSession,
        mouse: Vec2,
        actions: &mut Vec<OutfitAction>,
    ) {
        let rect = Rect::new(838.0, 80.0, 424.0, LOGICAL_HEIGHT - 98.0);
        draw_surface_with_title(
            rect,
            Some("Graftware Stores"),
            &panel_style(),
            TextStyle::new(16.0, dark::TEXT),
        );
        let content = rect.inset(12.0);
        let mut y = content.y + 40.0;

        let equipped = session.profile.equipped_item_ids();
        let slot_sel = self.selected_slot.as_ref();
        let creature = self
            .selected
            .and_then(|id| session.profile.roster.creature(id));

        for item in &session.profile.inventory.items {
            if equipped.contains(&item.id) {
                continue;
            }
            if y > content.bottom() - 44.0 {
                draw_ui_text_ex(
                    "…",
                    content.x,
                    y,
                    TextStyle::new(14.0, dark::TEXT_DIM).params(),
                );
                break;
            }
            let Some(def) = data.graftware.get(&item.def_id) else {
                continue;
            };
            let row = Rect::new(content.x, y, content.w, 44.0);
            let hovered = row.contains_point(mouse);
            draw_surface(
                row,
                &SurfaceStyle::new(if hovered {
                    Color::new(0.14, 0.17, 0.22, 1.0)
                } else {
                    Color::new(0.10, 0.115, 0.15, 1.0)
                }),
            );
            let elem_label = def.element.map(|e| e.display_name()).unwrap_or("Neutral");
            draw_ui_text_ex(
                &format!(
                    "{}  ·  {} {}",
                    def.name,
                    def.weight.display_name(),
                    def.kind.display_name()
                ),
                row.x + 10.0,
                row.y + 18.0,
                TextStyle::new(
                    14.0,
                    if item.is_usable() {
                        dark::TEXT
                    } else {
                        dark::TEXT_DIM
                    },
                )
                .params(),
            );
            draw_ui_text_ex(
                &format!(
                    "draw {} · min pwr {} · {}",
                    def.power_draw, def.min_power, elem_label
                ),
                row.x + 10.0,
                row.y + 36.0,
                TextStyle::new(12.0, dark::TEXT_DIM).params(),
            );

            if item.is_usable() {
                if let (Some((limb_id, slot)), Some(c)) = (slot_sel, creature) {
                    if hovered && is_mouse_button_released(MouseButton::Left) {
                        actions.push(OutfitAction::Equip {
                            creature: c.id,
                            limb_id: limb_id.clone(),
                            slot: *slot,
                            item: item.id,
                        });
                    }
                }
            } else {
                let cost = def.repair_cost();
                let btn = Rect::new(row.right() - 118.0, row.y + 7.0, 110.0, 30.0);
                if menu_button(
                    btn,
                    &format!("Repair {}", cost),
                    session.profile.inventory.scrip >= cost,
                    mouse,
                ) {
                    actions.push(OutfitAction::Repair(item.id));
                }
            }
            y += 50.0;
        }
    }
}

fn panel_style() -> SurfaceStyle {
    SurfaceStyle::new(Color::new(0.08, 0.085, 0.105, 0.97))
        .with_border(1.0, Color::new(0.38, 0.45, 0.58, 0.65))
        .with_header(36.0, Color::new(0.105, 0.12, 0.15, 1.0))
        .with_header_divider(1.0, Color::new(0.38, 0.45, 0.58, 0.4))
}

trait PairDeref {
    fn as_deref_pair(&self) -> Option<(&str, usize)>;
}

impl PairDeref for Option<(String, usize)> {
    fn as_deref_pair(&self) -> Option<(&str, usize)> {
        self.as_ref().map(|(s, i)| (s.as_str(), *i))
    }
}
