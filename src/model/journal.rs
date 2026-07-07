//! The rider's chronicle: an auto-generated log of what they have done, plus
//! the next thing the world is waiting on. Read in the codex Journal tab.

use crate::data::world::MapKind;
use crate::data::GameData;
use crate::model::worldstate::Verdict;
use crate::state::GameSession;
use serde::{Deserialize, Serialize};

const MAX_ENTRIES: usize = 120;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalEntry {
    /// Overworld step count when it happened — a rough clock for the chronicle.
    pub step: u64,
    pub text: String,
}

/// Append a chronicle line, trimming the oldest past the cap.
pub fn record(session: &mut GameSession, text: impl Into<String>) {
    let step = session.steps;
    session.journal.push(JournalEntry {
        step,
        text: text.into(),
    });
    let overflow = session.journal.len().saturating_sub(MAX_ENTRIES);
    if overflow > 0 {
        session.journal.drain(..overflow);
    }
}

/// The next objective, derived from the region the rider currently stands in.
/// This is the same read the old on-screen banner used, now surfaced only in
/// the Journal tab.
pub fn current_step(data: &GameData, session: &GameSession) -> String {
    let Some(map) = data.world.map(&session.location.map_id) else {
        return "Find your bearings.".to_owned();
    };
    if map.kind == MapKind::Factory {
        if let Some(fid) = &map.factory_id {
            let f = session.world_state.factory(fid);
            return if !f.heart_defeated {
                "Descend to the heart and silence it.".to_owned()
            } else {
                "The vats here are still. Nothing left to fight.".to_owned()
            };
        }
    }
    let Some(region) = data.world.region(&map.region) else {
        return "Follow the road.".to_owned();
    };
    let f = session.world_state.factory(&region.gestarium_id);
    let fname = data
        .factories
        .get(&region.gestarium_id)
        .map(|d| d.name.clone())
        .unwrap_or_else(|| "the factory".to_owned());

    if !f.heart_defeated {
        format!("{} still births war-units — raid its heart.", fname)
    } else if f.verdict.is_none() {
        format!("{} lies silent — return to pass judgment.", fname)
    } else if f.relapsed {
        "This region has relapsed — confront the keeper at the heart.".to_owned()
    } else if matches!(f.verdict, Some(Verdict::Reseed)) && !f.invested {
        "Revived, but untended — fund the Watch before prosperity forgets.".to_owned()
    } else {
        match f.verdict {
            Some(Verdict::Purge) => "At peace. Dead peace, but peace. Your verdict holds.".to_owned(),
            Some(Verdict::Bind) => "The factory answers to your hand now.".to_owned(),
            _ => "Thriving under your watch.".to_owned(),
        }
    }
}
