//! Quest definitions: NPC-offered objectives with one-shot rewards
//! (`game_design.md` §10 — the Hollow asks small, concrete things of the rider).

use serde::{Deserialize, Serialize};

/// What finishing a quest requires.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestObjectiveKind {
    /// Subdue wild creatures anywhere in the world.
    SubdueWild,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestObjective {
    pub kind: QuestObjectiveKind,
    /// How many of the tracked event must occur.
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestDef {
    pub id: String,
    pub name: String,
    /// Display name of the NPC who offers and closes the quest — used to point
    /// the player home when the objective is met.
    pub giver: String,
    /// One-line objective shown in the overworld tracker.
    pub tracker: String,
    pub objective: QuestObjective,
    #[serde(default)]
    pub reward_scrip: i64,
    #[serde(default)]
    pub reward_grafts: Vec<String>,
}

#[cfg(test)]
mod tests {
    use crate::data::GameData;

    /// Every quest must reference real reward grafts and a sane objective.
    #[test]
    fn quests_are_valid() {
        let data = GameData::load().unwrap();
        assert!(!data.quests.is_empty(), "no quests authored");
        for (_, q) in data.quests.iter() {
            assert!(q.objective.count > 0, "{}: zero-count objective", q.id);
            assert!(!q.tracker.is_empty(), "{}: empty tracker", q.id);
            for graft in &q.reward_grafts {
                assert!(
                    data.graftware.contains(graft),
                    "{}: unknown reward graft {}",
                    q.id,
                    graft
                );
            }
        }
    }
}
