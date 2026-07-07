//! Battle event stream — the engine's narration channel for UI, log lines,
//! animation triggers, and Wait-mode decision points.

use crate::combat::UnitId;

#[derive(Debug, Clone, PartialEq)]
pub enum BattleEvent {
    Hit {
        attacker: UnitId,
        target: UnitId,
        amount: f32,
        to_core: bool,
    },
    Miss {
        attacker: UnitId,
        target: UnitId,
    },
    LimbSevered {
        unit: UnitId,
        limb_name: String,
    },
    LimbRegrown {
        unit: UnitId,
        limb_name: String,
    },
    GraftDestroyed {
        unit: UnitId,
        graft_name: String,
    },
    GraftRejected {
        unit: UnitId,
        graft_name: String,
    },
    SalvageDropped {
        def_id: String,
    },
    CoreExposed {
        unit: UnitId,
    },
    CoreCracked {
        unit: UnitId,
    },
    BerserkStarted {
        unit: UnitId,
    },
    BerserkEnded {
        unit: UnitId,
    },
    HopStarted {
        from: Option<UnitId>,
        to: UnitId,
    },
    HopLanded {
        to: UnitId,
    },
    RiderExposed,
    Healed {
        source: UnitId,
        target: UnitId,
        amount: f32,
    },
    Shielded {
        unit: UnitId,
        amount: f32,
    },
    /// A consumable was used on `unit` — `label` is the floating callout.
    ItemUsed {
        unit: UnitId,
        label: String,
    },
    StanceChanged {
        unit: UnitId,
    },
    /// A Wait-mode decision point: the ridden creature has an action ready.
    RiddenActionReady {
        unit: UnitId,
    },
    BattleEnded,
}
