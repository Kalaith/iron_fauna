//! Player-side quest progression: which quests are accepted, how far along
//! their objective is, and which are finished. Definitions live in `data::quest`.

use crate::data::quest::QuestObjectiveKind;
use crate::data::GameData;
use crate::model::inventory::GraftCondition;
use crate::state::GameSession;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// A quest's lifecycle: not engaged → active (progressing) → ready (objective
/// met, awaiting turn-in) → done (reward claimed).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QuestLog {
    /// Accepted quests still in progress: id → current progress count.
    #[serde(default)]
    pub active: HashMap<String, u32>,
    /// Objective met, waiting for the giver to close it out.
    #[serde(default)]
    pub ready: HashSet<String>,
    /// Finished quests (reward already granted).
    #[serde(default)]
    pub done: HashSet<String>,
}

impl QuestLog {
    pub fn is_active(&self, id: &str) -> bool {
        self.active.contains_key(id)
    }

    pub fn is_ready(&self, id: &str) -> bool {
        self.ready.contains(id)
    }

    pub fn is_done(&self, id: &str) -> bool {
        self.done.contains(id)
    }

    /// True only if the quest has never been engaged in any way — the gate for
    /// showing the offer.
    pub fn is_untaken(&self, id: &str) -> bool {
        !self.is_active(id) && !self.is_ready(id) && !self.is_done(id)
    }

    /// The first quest worth surfacing in the HUD: a ready one takes priority
    /// (go turn it in), otherwise the first active one.
    pub fn tracked(&self) -> Option<&str> {
        self.ready
            .iter()
            .next()
            .or_else(|| self.active.keys().next())
            .map(|s| s.as_str())
    }
}

/// Accept a quest if it hasn't been engaged yet. Returns a notice line.
pub fn start(session: &mut GameSession, data: &GameData, quest_id: &str) -> Option<String> {
    if !session.quests.is_untaken(quest_id) {
        return None;
    }
    let def = data.quests.get(quest_id)?;
    let name = def.name.clone();
    session.quests.active.insert(quest_id.to_owned(), 0);
    crate::model::journal::record(session, format!("Took the bounty: {}.", name));
    Some(format!("New quest: {}", name))
}

/// Advance every active `SubdueWild` quest by `n` subdued creatures. Any quest
/// whose objective is now met moves to `ready`. Returns notice lines.
pub fn advance_subdue(session: &mut GameSession, data: &GameData, n: u32) -> Vec<String> {
    let mut notes = Vec::new();
    let mut completed = Vec::new();
    for (id, progress) in session.quests.active.iter_mut() {
        let Some(def) = data.quests.get(id) else {
            continue;
        };
        if def.objective.kind != QuestObjectiveKind::SubdueWild {
            continue;
        }
        *progress = (*progress + n).min(def.objective.count);
        if *progress >= def.objective.count {
            completed.push(id.clone());
            notes.push(format!(
                "Objective met: {} — return to {}",
                def.name, def.giver
            ));
        }
    }
    for id in completed {
        session.quests.active.remove(&id);
        session.quests.ready.insert(id);
    }
    notes
}

/// Turn in a ready quest, granting its reward. Returns notice lines.
pub fn complete(session: &mut GameSession, data: &GameData, quest_id: &str) -> Vec<String> {
    let mut notes = Vec::new();
    if !session.quests.is_ready(quest_id) {
        return notes;
    }
    let Some(def) = data.quests.get(quest_id).cloned() else {
        return notes;
    };
    session.quests.ready.remove(quest_id);
    session.quests.done.insert(quest_id.to_owned());
    crate::model::journal::record(session, format!("Saw through the bounty: {}.", def.name));
    if def.reward_scrip > 0 {
        session.profile.inventory.scrip += def.reward_scrip;
        notes.push(format!("Received {} scrip", def.reward_scrip));
    }
    for graft_id in &def.reward_grafts {
        session
            .profile
            .grant_graft(graft_id, GraftCondition::Intact);
        if let Some(g) = data.graftware.get(graft_id) {
            notes.push(format!("Received: {}", g.name));
        }
    }
    notes
}

/// HUD line for the tracked quest: "Objective (2/3)" or a turn-in prompt.
pub fn tracker_line(session: &GameSession, data: &GameData) -> Option<String> {
    let id = session.quests.tracked()?;
    let def = data.quests.get(id)?;
    if session.quests.is_ready(id) {
        return Some(format!("{} — see {}", def.name, def.giver));
    }
    let progress = session.quests.active.get(id).copied().unwrap_or(0);
    Some(format!(
        "{} ({}/{})",
        def.tracker, progress, def.objective.count
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quest_runs_offer_to_reward() {
        let data = GameData::load().unwrap();
        let mut session = GameSession::new_game(&data);
        let id = "morning_thinning";
        let count = data.quests.get(id).unwrap().objective.count;

        assert!(session.quests.is_untaken(id));
        assert!(start(&mut session, &data, id).is_some());
        assert!(session.quests.is_active(id));
        // Starting again is a no-op.
        assert!(start(&mut session, &data, id).is_none());

        // One short of the goal: still active, not ready.
        advance_subdue(&mut session, &data, count - 1);
        assert!(session.quests.is_active(id));
        assert!(!session.quests.is_ready(id));

        // The step that meets the objective flips it to ready.
        let notes = advance_subdue(&mut session, &data, 1);
        assert!(!notes.is_empty());
        assert!(session.quests.is_ready(id));

        let scrip_before = session.profile.inventory.scrip;
        let reward = complete(&mut session, &data, id);
        assert!(!reward.is_empty());
        assert!(session.quests.is_done(id));
        assert!(session.profile.inventory.scrip > scrip_before);
    }
}
