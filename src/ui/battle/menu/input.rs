//! Keyboard handling for the command menu: one `input_*` handler per
//! [`Screen`], dispatched from [`BattleScreen::handle_menu_input`]. Handlers
//! translate keys into `PlayerCommand`s and screen transitions only.

use super::{BattleScreen, RootCmd, Screen};
use crate::combat::{PlayerCommand, Side, Stance, WeaponRef};
use crate::data::GameData;
use macroquad::prelude::*;

impl BattleScreen {
    pub(in crate::ui::battle) fn handle_menu_input(&mut self, data: &GameData) {
        let up = is_key_pressed(KeyCode::Up);
        let down = is_key_pressed(KeyCode::Down);
        let confirm = is_key_pressed(KeyCode::Enter)
            || is_key_pressed(KeyCode::KpEnter)
            || is_key_pressed(KeyCode::Z);
        let back = is_key_pressed(KeyCode::Escape) || is_key_pressed(KeyCode::Backspace);

        match self.menu.screen {
            Screen::Root => self.input_root(data, up, down, confirm, back),
            Screen::Weapon => self.input_weapon(data, up, down, confirm, back),
            Screen::Target => self.input_target(data, up, down, confirm, back),
            Screen::Utility => self.input_utility(data, up, down, confirm, back),
            Screen::Item => self.input_item(data, up, down, confirm, back),
            Screen::Hop => self.input_hop(data, up, down, confirm, back),
            Screen::Stance => self.input_stance(data, up, down, confirm, back),
        }
    }

    fn step_cursor(cursor: &mut usize, len: usize, up: bool, down: bool) {
        if len == 0 {
            *cursor = 0;
            return;
        }
        if up {
            *cursor = (*cursor + len - 1) % len;
        }
        if down {
            *cursor = (*cursor + 1) % len;
        }
    }

    fn input_root(&mut self, data: &GameData, up: bool, down: bool, confirm: bool, back: bool) {
        let cmds = self.root_commands(data);
        Self::step_cursor(&mut self.menu.cursor, cmds.len(), up, down);
        if back {
            // Esc on the root is "let it ride".
            self.menu.open = false;
            return;
        }
        if !confirm {
            return;
        }
        let Some(&cmd) = cmds.get(self.menu.cursor) else {
            return;
        };
        match cmd {
            RootCmd::Attack => {
                self.menu.screen = Screen::Weapon;
                self.menu.cursor = 0;
            }
            RootCmd::Utility => {
                self.menu.screen = Screen::Utility;
                self.menu.cursor = 0;
            }
            RootCmd::Reinforce => {
                if self.battle.command(data, PlayerCommand::Reinforce) {
                    self.after_command();
                }
            }
            RootCmd::Regrow => {
                if let Some(id) = self.battle.ridden_unit() {
                    let limb = self.battle.units[id].limbs.iter().position(|l| l.severed);
                    if let Some(limb) = limb {
                        if self.battle.command(data, PlayerCommand::Regrow { limb }) {
                            self.after_command();
                        }
                    }
                }
            }
            RootCmd::Item => {
                self.menu.screen = Screen::Item;
                self.menu.cursor = 0;
            }
            RootCmd::Hop => {
                self.menu.screen = Screen::Hop;
                self.menu.cursor = 0;
            }
            RootCmd::Stance => {
                self.menu.screen = Screen::Stance;
                self.menu.cursor = 0;
            }
            RootCmd::Pass => {
                self.menu.open = false;
            }
        }
    }

    fn input_weapon(&mut self, data: &GameData, up: bool, down: bool, confirm: bool, back: bool) {
        let list = self.weapon_list(data);
        Self::step_cursor(&mut self.menu.cursor, list.len(), up, down);
        if back {
            self.menu.screen = Screen::Root;
            return;
        }
        if !confirm {
            return;
        }
        let Some(&weapon) = list.get(self.menu.cursor) else {
            return;
        };
        if !self.weapon_ready(data, weapon) {
            return; // on cooldown / no vigor — pick another
        }
        self.ensure_valid_target();
        self.menu.pending_weapon = Some(weapon);
        self.menu.screen = Screen::Target;
        self.menu.part_cursor = 0;
    }

    fn input_target(&mut self, data: &GameData, up: bool, down: bool, confirm: bool, back: bool) {
        // Left/Right (or Tab) switch which enemy is in the crosshair.
        if is_key_pressed(KeyCode::Left)
            || is_key_pressed(KeyCode::Right)
            || is_key_pressed(KeyCode::Tab)
        {
            let enemies = self.battle.alive_on(Side::Enemy);
            if !enemies.is_empty() {
                let cur = enemies.iter().position(|&e| e == self.target).unwrap_or(0);
                let next = if is_key_pressed(KeyCode::Left) {
                    (cur + enemies.len() - 1) % enemies.len()
                } else {
                    (cur + 1) % enemies.len()
                };
                self.target = enemies[next];
                self.menu.part_cursor = 0;
            }
        }
        let parts = self.part_list();
        Self::step_cursor(&mut self.menu.part_cursor, parts.len(), up, down);
        if back {
            self.menu.screen = Screen::Weapon;
            return;
        }
        if !confirm {
            return;
        }
        let called = parts.get(self.menu.part_cursor).and_then(|p| p.called());
        let Some(weapon) = self.menu.pending_weapon else {
            self.menu.screen = Screen::Root;
            return;
        };
        let cmd = match weapon {
            WeaponRef::Natural => PlayerCommand::NaturalAttack {
                target: self.target,
                called,
            },
            WeaponRef::Mount(mount) => PlayerCommand::Fire {
                mount,
                target: self.target,
                called,
            },
        };
        if self.battle.command(data, cmd) {
            self.after_command();
        }
    }

    fn input_utility(&mut self, data: &GameData, up: bool, down: bool, confirm: bool, back: bool) {
        let list = self.utility_list(data);
        Self::step_cursor(&mut self.menu.cursor, list.len(), up, down);
        if back {
            self.menu.screen = Screen::Root;
            return;
        }
        if !confirm {
            return;
        }
        if let Some(&mount) = list.get(self.menu.cursor) {
            if self
                .battle
                .command(data, PlayerCommand::TriggerUtility { mount, ally: None })
            {
                self.after_command();
            }
        }
    }

    fn input_item(&mut self, data: &GameData, up: bool, down: bool, confirm: bool, back: bool) {
        let potions = self.battle.usable_potions(data);
        let ammo = self.battle.usable_ammo(data);
        let len = potions.len() + ammo.len();
        Self::step_cursor(&mut self.menu.cursor, len.max(1), up, down);
        if back {
            self.menu.screen = Screen::Root;
            return;
        }
        if !confirm {
            return;
        }
        let i = self.menu.cursor;
        let used = if i < potions.len() {
            self.battle.use_potion(data, &potions[i].0.clone())
        } else if let Some((def_id, _)) = ammo.get(i - potions.len()).cloned() {
            self.load_first_weapon(data, &def_id)
        } else {
            false
        };
        if used {
            self.after_command();
        }
    }

    fn input_hop(&mut self, data: &GameData, up: bool, down: bool, confirm: bool, back: bool) {
        let list = self.hop_targets();
        Self::step_cursor(&mut self.menu.cursor, list.len(), up, down);
        if back && !self.menu.forced_hop {
            self.menu.screen = Screen::Root;
            return;
        }
        if !confirm {
            return;
        }
        if let Some(&to) = list.get(self.menu.cursor) {
            // BeginHop routes through the engine even while the rider is exposed.
            if self.battle.command(data, PlayerCommand::BeginHop { to }) {
                self.after_command();
                self.menu.forced_hop = false;
            }
        }
    }

    fn input_stance(&mut self, data: &GameData, up: bool, down: bool, confirm: bool, back: bool) {
        let allies = self.battle.alive_on(Side::Player);
        Self::step_cursor(&mut self.menu.cursor, allies.len(), up, down);
        if back {
            self.menu.screen = Screen::Root;
            return;
        }
        if !confirm {
            return;
        }
        if let Some(&unit) = allies.get(self.menu.cursor) {
            let stance = match self.battle.units[unit].stance {
                Stance::Aggressive => Stance::Defensive,
                Stance::Defensive => Stance::Aggressive,
            };
            // Orders are free to set — stay on the panel, don't consume the turn.
            let _ = self
                .battle
                .command(data, PlayerCommand::SetStance { unit, stance });
        }
    }
}
