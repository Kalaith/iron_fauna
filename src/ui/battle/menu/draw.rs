//! Rendering for the command menu: the panel chrome plus per-screen row
//! builders. Pure view code — reads state, never mutates it.

use super::{BattleScreen, Part, Screen};
use crate::combat::{Side, WeaponRef};
use crate::data::GameData;
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::draw_ui_text_ex;

impl BattleScreen {
    pub(in crate::ui::battle) fn draw_menu(&self, data: &GameData) {
        let (title, rows, cursor, hint) = self.menu_rows(data);
        let panel_h =
            66.0 + rows.len().max(1) as f32 * 26.0 + if hint.is_some() { 22.0 } else { 0.0 };
        let rect = Rect::new(
            24.0,
            crate::ui::LOGICAL_HEIGHT - panel_h - 24.0,
            430.0,
            panel_h,
        );
        draw_surface(
            rect,
            &SurfaceStyle::new(Color::new(0.05, 0.07, 0.09, 0.96))
                .with_border(1.5, Color::new(0.4, 0.6, 0.75, 0.85)),
        );
        draw_ui_text_ex(
            &title,
            rect.x + 16.0,
            rect.y + 28.0,
            TextStyle::new(18.0, Color::new(0.85, 0.92, 0.98, 1.0)).params(),
        );
        let mut y = rect.y + 56.0;
        if rows.is_empty() {
            draw_ui_text_ex(
                "— nothing available —",
                rect.x + 20.0,
                y,
                TextStyle::new(15.0, dark::TEXT_DIM).params(),
            );
        }
        for (i, (label, enabled)) in rows.iter().enumerate() {
            let selected = i == cursor;
            if selected {
                draw_rectangle(
                    rect.x + 8.0,
                    y - 16.0,
                    rect.w - 16.0,
                    24.0,
                    Color::new(0.2, 0.32, 0.42, 0.9),
                );
            }
            let color = if !enabled {
                dark::TEXT_DIM
            } else if selected {
                Color::new(0.98, 0.98, 0.9, 1.0)
            } else {
                dark::TEXT
            };
            draw_ui_text_ex(
                &format!("{} {}", if selected { ">" } else { " " }, label),
                rect.x + 16.0,
                y,
                TextStyle::new(16.0, color).params(),
            );
            y += 26.0;
        }
        if let Some(hint) = hint {
            draw_ui_text_ex(
                &hint,
                rect.x + 16.0,
                y + 4.0,
                TextStyle::new(13.0, dark::TEXT_DIM).params(),
            );
        }
    }

    /// (title, [(row label, enabled)], cursor index, optional hint) for the
    /// current screen.
    fn menu_rows(&self, data: &GameData) -> (String, Vec<(String, bool)>, usize, Option<String>) {
        match self.menu.screen {
            Screen::Root => {
                let name = self
                    .battle
                    .ridden_unit()
                    .map(|id| self.battle.units[id].spec_name.as_str())
                    .unwrap_or("—");
                let rows = self
                    .root_commands(data)
                    .into_iter()
                    .map(|c| (c.label().to_owned(), true))
                    .collect();
                (
                    format!("{} — orders", name),
                    rows,
                    self.menu.cursor,
                    Some("[Up/Dn] choose · [Enter] confirm · [Esc] let it ride".to_owned()),
                )
            }
            Screen::Weapon => {
                let rows = self
                    .weapon_list(data)
                    .into_iter()
                    .map(|w| self.weapon_row(data, w))
                    .collect();
                (
                    "Attack with…".to_owned(),
                    rows,
                    self.menu.cursor,
                    Some("[Up/Dn] choose · [Enter] aim · [Esc] back".to_owned()),
                )
            }
            Screen::Target => {
                let tname = self
                    .battle
                    .units
                    .get(self.target)
                    .map(|u| u.spec_name.clone())
                    .unwrap_or_default();
                let rows = self
                    .part_list()
                    .into_iter()
                    .map(|p| (self.part_label(data, p), true))
                    .collect();
                (
                    format!("Strike {} —", tname),
                    rows,
                    self.menu.part_cursor,
                    Some(
                        "[<-/->] switch foe · [Up/Dn] pick spot · [Enter] fire · [Esc] back"
                            .to_owned(),
                    ),
                )
            }
            Screen::Utility => {
                let rows = self
                    .utility_list(data)
                    .into_iter()
                    .map(|m| {
                        let u = &self.battle.units[self.battle.ridden_unit().unwrap_or(0)];
                        let ready = u.mounts[m].cooldown <= 0.0;
                        let name = self.graft_name(data, &u.mounts[m].def_id).to_owned();
                        let status = if ready {
                            "ready".to_owned()
                        } else {
                            format!("{:.1}s", u.mounts[m].cooldown)
                        };
                        (format!("{} — {}", name, status), ready)
                    })
                    .collect();
                (
                    "Utility".to_owned(),
                    rows,
                    self.menu.cursor,
                    Some("Enter use · Esc back".to_owned()),
                )
            }
            Screen::Item => {
                let mut rows: Vec<(String, bool)> = Vec::new();
                let ready = self.battle.potion_ready();
                for (def_id, count) in self.battle.usable_potions(data) {
                    let name = data
                        .items
                        .get(&def_id)
                        .map(|d| d.name.as_str())
                        .unwrap_or("?");
                    rows.push((format!("{} ×{}  (potion)", name, count), ready));
                }
                for (def_id, count) in self.battle.usable_ammo(data) {
                    let name = data
                        .items
                        .get(&def_id)
                        .map(|d| d.name.as_str())
                        .unwrap_or("?");
                    rows.push((format!("{} ×{}  (load weapon)", name, count), true));
                }
                (
                    "Items".to_owned(),
                    rows,
                    self.menu.cursor,
                    Some("Enter use · Esc back".to_owned()),
                )
            }
            Screen::Hop => {
                let rows = self
                    .hop_targets()
                    .into_iter()
                    .map(|id| {
                        let u = &self.battle.units[id];
                        (
                            format!(
                                "{}  (core {:.0}%)",
                                u.spec_name,
                                u.core_hp / u.core_max * 100.0
                            ),
                            true,
                        )
                    })
                    .collect();
                let hint = if self.menu.forced_hop {
                    "Rider exposed — choose a mount · Enter ride"
                } else {
                    "Enter ride · Esc back"
                };
                (
                    "Ride which creature?".to_owned(),
                    rows,
                    self.menu.cursor,
                    Some(hint.to_owned()),
                )
            }
            Screen::Stance => {
                let rows = self
                    .battle
                    .alive_on(Side::Player)
                    .into_iter()
                    .map(|id| {
                        let u = &self.battle.units[id];
                        (
                            format!("{} — {}", u.spec_name, u.stance.display_name()),
                            true,
                        )
                    })
                    .collect();
                (
                    "Standing orders".to_owned(),
                    rows,
                    self.menu.cursor,
                    Some("Enter toggle · Esc back".to_owned()),
                )
            }
        }
    }

    fn weapon_row(&self, data: &GameData, weapon: WeaponRef) -> (String, bool) {
        let ready = self.weapon_ready(data, weapon);
        let Some(id) = self.battle.ridden_unit() else {
            return ("—".to_owned(), false);
        };
        let u = &self.battle.units[id];
        match weapon {
            WeaponRef::Natural => {
                let status = if u.natural_cooldown <= 0.0 {
                    "ready".to_owned()
                } else {
                    format!("{:.1}s", u.natural_cooldown)
                };
                (format!("Natural strike — {}", status), ready)
            }
            WeaponRef::Mount(m) => {
                let name = self.graft_name(data, &u.mounts[m].def_id).to_owned();
                let dmg = data
                    .graftware
                    .get(&u.mounts[m].def_id)
                    .map(|d| d.damage)
                    .unwrap_or(0.0);
                let status = if u.mounts[m].cooldown <= 0.0 {
                    "ready".to_owned()
                } else {
                    format!("{:.1}s", u.mounts[m].cooldown)
                };
                (format!("{} ({:.0} dmg) — {}", name, dmg, status), ready)
            }
        }
    }

    fn part_label(&self, data: &GameData, part: Part) -> String {
        let Some(t) = self.battle.units.get(self.target) else {
            return "—".to_owned();
        };
        match part {
            Part::Center => "center mass".to_owned(),
            Part::Limb(li) => t
                .limbs
                .get(li)
                .map(|l| t.limb_def(data, l).name.clone())
                .unwrap_or_else(|| "limb".to_owned()),
            Part::Mount(mi) => match t.mounts.get(mi) {
                Some(m) => format!(
                    "{} ({:.0}%)",
                    self.graft_name(data, &m.def_id),
                    (m.graft_hp / m.graft_hp_max * 100.0).clamp(0.0, 100.0)
                ),
                None => "graft".to_owned(),
            },
        }
    }
}
