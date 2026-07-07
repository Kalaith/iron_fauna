//! Story progression: flag-gated dialogue selection and one-shot rewards.

use crate::data::world::{DialogueCond, DialogueRule, NpcDef};
use crate::data::GameData;
use crate::model::inventory::GraftCondition;
use crate::model::worldstate::Verdict;
use crate::state::GameSession;

fn verdict_matches(name: &str, verdict: Option<Verdict>) -> bool {
    matches!(
        (name, verdict),
        ("Purge", Some(Verdict::Purge))
            | ("Reseed", Some(Verdict::Reseed))
            | ("Bind", Some(Verdict::Bind))
    )
}

pub fn cond_passes(cond: &DialogueCond, session: &GameSession) -> bool {
    if !cond
        .flags_all
        .iter()
        .all(|f| session.story_flags.contains(f))
    {
        return false;
    }
    if cond
        .flags_none
        .iter()
        .any(|f| session.story_flags.contains(f))
    {
        return false;
    }
    if let Some(factory) = &cond.heart_defeated {
        if !session.world_state.factory(factory).heart_defeated {
            return false;
        }
    }
    if let Some((factory, verdict)) = &cond.verdict {
        if !verdict_matches(verdict, session.world_state.verdict(factory)) {
            return false;
        }
    }
    if let Some((factory, wanted)) = &cond.relapsed {
        if session.world_state.factory(factory).relapsed != *wanted {
            return false;
        }
    }
    true
}

/// Picks the first dialogue rule whose condition passes, falling back to the
/// NPC's plain `lines`.
pub fn select_dialogue<'a>(
    npc: &'a NpcDef,
    session: &GameSession,
) -> Option<DialogueSelection<'a>> {
    for rule in &npc.dialogue {
        let passes = rule
            .when
            .as_ref()
            .map(|c| cond_passes(c, session))
            .unwrap_or(true);
        if passes {
            return Some(DialogueSelection {
                lines: &rule.lines,
                rule: Some(rule),
            });
        }
    }
    if npc.lines.is_empty() {
        None
    } else {
        Some(DialogueSelection {
            lines: &npc.lines,
            rule: None,
        })
    }
}

pub struct DialogueSelection<'a> {
    pub lines: &'a [String],
    pub rule: Option<&'a DialogueRule>,
}

/// Applies a rule's effects (flags, rewards). Rewards are inherently
/// one-shot: gate them behind a flag in `flags_none` + `set_flags`.
/// Returns notification lines.
pub fn apply_dialogue_effects(
    rule: &DialogueRule,
    session: &mut GameSession,
    data: &GameData,
) -> Vec<String> {
    let mut notes = Vec::new();
    for flag in &rule.set_flags {
        session.story_flags.insert(flag.clone());
    }
    if rule.give_scrip > 0 {
        session.profile.inventory.scrip += rule.give_scrip;
        notes.push(format!("Received {} scrip", rule.give_scrip));
    }
    for def_id in &rule.give_grafts {
        session.profile.grant_graft(def_id, GraftCondition::Intact);
        if let Some(def) = data.graftware.get(def_id) {
            notes.push(format!("Received: {}", def.name));
        }
    }
    notes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::world::{DialogueCond, DialogueRule, NpcDef};
    use crate::model::worldstate::Verdict;

    fn npc_with_rules(rules: Vec<DialogueRule>) -> NpcDef {
        NpcDef {
            id: "test".into(),
            x: 0,
            y: 0,
            name: "Test".into(),
            lines: vec!["fallback".into()],
            dialogue: rules,
        }
    }

    fn rule(when: Option<DialogueCond>, line: &str) -> DialogueRule {
        DialogueRule {
            when,
            lines: vec![line.to_owned()],
            set_flags: vec![],
            give_scrip: 0,
            give_grafts: vec![],
        }
    }

    #[test]
    fn dialogue_reacts_to_flags_and_verdicts() {
        let data = crate::data::GameData::load().unwrap();
        let mut session = GameSession::new_game(&data);
        let npc = npc_with_rules(vec![
            rule(
                Some(DialogueCond {
                    verdict: Some(("the_cradle".into(), "Purge".into())),
                    ..Default::default()
                }),
                "after purge",
            ),
            rule(
                Some(DialogueCond {
                    flags_all: vec!["met".into()],
                    ..Default::default()
                }),
                "again",
            ),
        ]);

        // No flags, no verdict: falls through to plain lines.
        assert_eq!(
            select_dialogue(&npc, &session).unwrap().lines[0],
            "fallback"
        );

        session.story_flags.insert("met".into());
        assert_eq!(select_dialogue(&npc, &session).unwrap().lines[0], "again");

        // A verdict outranks the flag rule (listed first).
        session.world_state.factory_mut("the_cradle").verdict = Some(Verdict::Purge);
        assert_eq!(
            select_dialogue(&npc, &session).unwrap().lines[0],
            "after purge"
        );
    }

    #[test]
    fn effects_grant_once_when_gated_by_flags() {
        let data = crate::data::GameData::load().unwrap();
        let mut session = GameSession::new_game(&data);
        let reward = DialogueRule {
            when: Some(DialogueCond {
                flags_none: vec!["paid".into()],
                ..Default::default()
            }),
            lines: vec!["here, take this".into()],
            set_flags: vec!["paid".into()],
            give_scrip: 100,
            give_grafts: vec!["spark_coil".into()],
        };
        let npc = npc_with_rules(vec![reward]);

        let scrip_before = session.profile.inventory.scrip;
        let items_before = session.profile.inventory.items.len();
        let selection = select_dialogue(&npc, &session).unwrap();
        let rule = selection.rule.unwrap().clone();
        let notes = apply_dialogue_effects(&rule, &mut session, &data);
        assert_eq!(session.profile.inventory.scrip, scrip_before + 100);
        assert_eq!(session.profile.inventory.items.len(), items_before + 1);
        assert_eq!(notes.len(), 2);

        // Flag now set: the reward rule no longer matches.
        assert_eq!(
            select_dialogue(&npc, &session).unwrap().lines[0],
            "fallback"
        );
    }
}
