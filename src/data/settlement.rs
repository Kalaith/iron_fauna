//! Settlement definitions: shops and duelling rings
//! (`game_design.md` §10 — practice and staked NPC duels).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShopEntry {
    pub graft: String,
    /// Price override; defaults to the graft's base value.
    #[serde(default)]
    pub price: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuelGraftFit {
    pub limb: String,
    pub slot: usize,
    pub graft: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuelUnitDef {
    pub species: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub grafts: Vec<DuelGraftFit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuelistDef {
    pub id: String,
    pub name: String,
    pub blurb: String,
    /// Ladder rank required to challenge (0 = open to anyone).
    #[serde(default)]
    pub rank_req: u32,
    /// Practice duels risk nothing; staked duels wager parts.
    pub practice: bool,
    pub reward_scrip: i64,
    /// Staked lane: the part this duelist puts up.
    #[serde(default)]
    pub stake: Option<String>,
    /// Staked lane: minimum base value of the part you must put up.
    #[serde(default)]
    pub min_stake_value: i64,
    pub party: Vec<DuelUnitDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementDef {
    pub id: String,
    pub name: String,
    pub region: String,
    pub description: String,
    #[serde(default)]
    pub shop: Vec<ShopEntry>,
    #[serde(default)]
    pub duelists: Vec<DuelistDef>,
}

impl SettlementDef {
    pub fn duelist(&self, id: &str) -> Option<&DuelistDef> {
        self.duelists.iter().find(|d| d.id == id)
    }
}

#[cfg(test)]
mod tests {
    use crate::combat::unit::{BattleUnit, UnitSpec};
    use crate::combat::{Side, Stance};
    use crate::data::GameData;

    /// Every duelist party must actually assemble into legal battle units —
    /// species exist, limbs exist, weight classes fit.
    #[test]
    fn settlements_are_valid() {
        let data = GameData::load().unwrap();
        assert!(!data.settlements.is_empty());

        for (_, s) in data.settlements.iter() {
            assert!(
                data.world.region(&s.region).is_some(),
                "{}: bad region",
                s.id
            );
            for entry in &s.shop {
                assert!(
                    data.graftware.contains(&entry.graft),
                    "{}: unknown shop graft {}",
                    s.id,
                    entry.graft
                );
            }
            for duelist in &s.duelists {
                assert!(!duelist.party.is_empty(), "{}: empty party", duelist.id);
                if !duelist.practice {
                    let stake = duelist.stake.as_ref().unwrap_or_else(|| {
                        panic!("{}: staked duelist without a stake", duelist.id)
                    });
                    assert!(
                        data.graftware.contains(stake),
                        "{}: unknown stake {}",
                        duelist.id,
                        stake
                    );
                    assert!(duelist.min_stake_value > 0, "{}: free stake", duelist.id);
                }
                for unit in &duelist.party {
                    let spec = UnitSpec {
                        species_id: unit.species.clone(),
                        name: duelist.name.clone(),
                        side: Side::Enemy,
                        creature_id: None,
                        bond: 0.0,
                        stance: Stance::Aggressive,
                        grafts: unit
                            .grafts
                            .iter()
                            .map(|g| (g.limb.clone(), g.slot, g.graft.clone(), None))
                            .collect(),
                    };
                    BattleUnit::build(&spec, &data)
                        .unwrap_or_else(|err| panic!("{}: illegal duel unit: {}", duelist.id, err));
                }
            }
        }
    }
}
