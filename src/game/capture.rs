//! Headless screenshot-harness scene seeding. Kept out of the main game loop
//! so `game.rs` stays focused on live play (`docs/screenshot_capture_harness_guide.md`).

use super::{Game, Mode, ReturnTo};
use crate::combat::unit::UnitSpec;
use crate::combat::{BattleContext, Side, Stance};
use crate::model::creature::CreatureOrigin;
use crate::model::worldstate::Verdict;
use crate::state::{Location, PaceSetting};
use crate::ui::bestiary::BestiaryScreen;
use crate::ui::ledger::LedgerScreen;
use crate::ui::outfit::OutfitScreen;
use crate::ui::overworld::OverworldScreen;
use crate::ui::settlement::{SettlementScreen, SettlementView};
use crate::ui::verdict::{FactoryScreenKind, VerdictScreen};

impl Game {
    /// Seeds a named state for the headless screenshot harness.
    pub fn begin_capture_scene(&mut self, scene: &str) {
        match scene {
            "overworld" => {
                self.return_to = ReturnTo::Overworld;
                self.mode = Mode::Overworld(Box::new(OverworldScreen::new(&self.session)));
            }
            "fernhollow_quest" => {
                // Accept the town bounty so the quest tracker is visible.
                let _ =
                    crate::model::quest::start(&mut self.session, &self.data, "morning_thinning");
                self.return_to = ReturnTo::Overworld;
                self.mode = Mode::Overworld(Box::new(OverworldScreen::new(&self.session)));
            }
            "battle" => {
                self.return_to = ReturnTo::Menu;
                self.start_dev_battle();
            }
            "battle_aim" => {
                self.return_to = ReturnTo::Menu;
                self.session.battles_fought = 2; // wave with an armed war-unit
                self.start_dev_battle();
                if let Mode::Battle(screen) = &mut self.mode {
                    screen.force_aim_capture(&self.data);
                }
            }
            "battle_juice" => {
                // Active pace so combat runs freely and floating text appears.
                self.return_to = ReturnTo::Menu;
                self.session.pace = PaceSetting::Active;
                self.session.battles_fought = 2;
                self.start_dev_battle();
            }
            "outfit" => {
                self.mode = Mode::Outfit(OutfitScreen {
                    selected: self.session.profile.roster.party.first().copied(),
                    selected_slot: None,
                });
            }
            "outfit_bare" => {
                // Strip the starter's loadout so the portrait shows the bare
                // war-body — the "before" to the outfit scene's "after".
                for c in &mut self.session.profile.roster.creatures {
                    c.loadout.clear();
                }
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
            "factory" => {
                self.session.location = Location {
                    map_id: "cradle_f1".to_owned(),
                    x: 14,
                    y: 13,
                };
                self.return_to = ReturnTo::Overworld;
                self.mode = Mode::Overworld(Box::new(OverworldScreen::new(&self.session)));
            }
            "verdict" => {
                self.mode = Mode::FactoryHeart(VerdictScreen {
                    factory_id: "the_cradle".to_owned(),
                    kind: FactoryScreenKind::Verdict,
                });
            }
            "endgame" => {
                self.session.location = Location {
                    map_id: "ruin_field".to_owned(),
                    x: 15,
                    y: 8,
                };
                self.return_to = ReturnTo::Overworld;
                self.mode = Mode::Overworld(Box::new(OverworldScreen::new(&self.session)));
            }
            "codex" => self.codex_scene(crate::ui::codex::CodexTab::Status),
            "codex_corelings" => self.codex_scene(crate::ui::codex::CodexTab::Corelings),
            "codex_party" => self.codex_scene(crate::ui::codex::CodexTab::Party),
            "codex_quests" => self.codex_scene(crate::ui::codex::CodexTab::Quests),
            "codex_journal" => self.codex_scene(crate::ui::codex::CodexTab::Journal),
            "bestiary" => {
                // Seed a partial collection so the screen shows both states.
                for sp in [
                    "weavil",
                    "kestrelle",
                    "pangol",
                    "cervolt",
                    "toxole",
                    "tembolo",
                ] {
                    self.session
                        .profile
                        .spawn_creature(&self.data, sp, CreatureOrigin::Wild);
                }
                self.mode = Mode::Bestiary(BestiaryScreen);
            }
            "ledger" => {
                let verdicts = [
                    ("the_cradle", Verdict::Reseed, false),
                    ("the_font", Verdict::Purge, false),
                    ("the_spire", Verdict::Bind, false),
                    ("the_kiln", Verdict::Reseed, true),
                    ("the_bloom", Verdict::Purge, false),
                    ("the_maw", Verdict::Reseed, false),
                ];
                for (id, v, relapsed) in verdicts {
                    let s = self.session.world_state.factory_mut(id);
                    s.heart_defeated = true;
                    s.verdict = Some(v);
                    s.relapsed = relapsed;
                }
                self.mode = Mode::Ledger(LedgerScreen);
            }
            _ => self.mode = Mode::Menu,
        }
    }

    /// Seeds a party, bounty, verdict, and chronicle, then opens the codex on
    /// the given tab so each tab has content to show.
    fn codex_scene(&mut self, tab: crate::ui::codex::CodexTab) {
        for sp in ["weavil", "kestrelle", "pangol"] {
            self.session
                .profile
                .spawn_creature(&self.data, sp, CreatureOrigin::Wild);
        }
        let _ = crate::model::quest::start(&mut self.session, &self.data, "morning_thinning");
        self.session
            .world_state
            .factory_mut("the_cradle")
            .heart_defeated = true;
        self.session.world_state.factory_mut("the_cradle").verdict = Some(Verdict::Reseed);
        crate::model::journal::record(&mut self.session, "Silenced the Cradle's heart.");
        crate::model::journal::record(&mut self.session, "Chose to reseed the meadow.");
        let mut screen = crate::ui::codex::CodexScreen::new(&self.session, None);
        screen.tab = tab;
        self.mode = Mode::Codex(Box::new(screen));
    }

    /// A deterministic sample encounter for engine testing from the menu.
    pub(super) fn start_dev_battle(&mut self) {
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
