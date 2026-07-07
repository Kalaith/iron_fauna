# IRON FAUNA

A darker, survival-framed 2D creature-collector RPG: catch gentle wild creatures,
graft living weapons onto them, and ride them into battle to hold back a world
whose derelict bio-factories still grow their own monsters. Region by region,
decide whether each factory is destroyed, revived, or claimed — and live with
the world your verdicts add up to.

Built with Rust + Macroquad on `macroquad-toolkit` (WebGL + native Windows).

## The pitch

The old world built **Gestaria** — bio-factories that grow creatures already
armed, born with the gun grown on. The war ended; the factories didn't stop.
You're an orphan rider with no family to lose, so the settlements send you out
to thin what the Gestaria keep birthing.

You don't kill the creatures. You **crack the war-body shell** and free the small,
frightened **core** inside — then you graft salvaged weapons onto your own mounts
and ride them into the next fight. Every core is a cute thing you chose to protect
or to spend. When you reach a factory's heart you pass a **verdict** on it —
**Purge** it dead-safe, **Reseed** it back to life, or **Bind** it and grow your
own — and the region reshapes around what you decided. Revive a place and leave
it untended and it can **relapse**: someone waters the old poison, and you go back
to face the keeper you made possible.

## What's in it

- **Connected overworld** (`game_design.md` §8) — grid movement across linked tile
  maps, tall-grass encounters, NPC dialogue, and doors into settlements and
  factory floors. Your home town **Fernhollow** has enterable buildings (bench,
  supply post, duelling ring), NPCs, and a quartermaster who hands out a bounty.
- **Semi-real-time combat** (`combat.md`) — a side-view battlefield with **Wait**
  (auto-pause at decisions) or **Active** pacing, a 6-slot party budget, rider
  possession with a per-loadout **Boost**, hopping between mounts, standing-order
  stances, and **called shots** that sever specific limbs or snipe grafts.
- **Chassis + graftware** (`creature.md`) — each species is a chassis (Power, Size,
  Speed, Element, limb layout); graftware mounts onto limb slots under a **Power
  Draw vs. Strain** budget, with element synergy. All combat numbers derive from
  balance curves in `assets/data/balance.json`.
- **Procedural creatures** — every creature is drawn from its chassis by
  `ui/creature_art.rs` (element → palette, archetype/size/temperament → silhouette,
  limbs → graft mounts), so the **before/after of grafting** is visible on the
  bench, in the bestiary (`chassis → grafted` preview), and on the battlefield.
- **Factories & verdicts** — raid a Gestarium's heart, then Purge / Reseed / Bind
  it; the world ledger tracks every judgment and its fallout (relapse, stewardship).
- **Settlements** — a grafting bench, a supply post (buy/sell/repair, fund the
  watch), and a **duelling ring** with practice bouts and staked wagers.
- **Quests** — data-driven bounties offered through NPC dialogue, tracked in the
  overworld HUD, turned in for scrip and parts.
- **Bestiary** — every species catalogued; caught ones shown in full, the rest as
  silhouettes to chase.

## Controls

**Overworld:** `WASD` / arrows move · `Space` / `Enter` interact (talk, enter a
door) · `Esc` menu.

**Battle:** `Q`/`W`/`E`/`R` fire weapon mounts · `A` natural attack · `S` utility ·
`D` reinforce · `G` regrow a severed limb · `H` hop to another mount ·
`C` aim mode (arrows pick a limb region, `X` re-centres) · `Tab` / `Up` / `Down`
switch target · `1`–`6` toggle a creature's stance · `P` pause · `Space` let a
Wait-paused fight ride.

## Run / test

```powershell
cargo run          # native debug
cargo test         # unit + simulation/balance tests
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt -- --check
.\publish.ps1      # full build + deploy (WebGL + Windows)
```

## Screenshot harness

Headless scene capture for visual verification (no interactive input required):

```powershell
.\scripts\capture_ui.ps1 -Scenes <scene1,scene2>
```

Scenes: `overworld`, `fernhollow_quest`, `battle`, `battle_aim`, `battle_juice`,
`outfit`, `outfit_bare`, `settlement`, `factory`, `verdict`, `bestiary`, `ledger`,
`endgame`. Wired through the `IRON_FAUNA_CAPTURE_*` env vars from
`macroquad_toolkit::capture`; captures land in `docs/verification/`.

## Design documents

The design canon lives in this directory and drives implementation:

- `game_design.md` — master GDD: high concept, pillars, creature anatomy, combat
  contexts, factories, verdict system, duelling, progression.
- `combat.md` — combat system: Wait/Active pause, 6-slot party budget, rider
  possession/Boost/hopping, standing orders, called shots.
- `creature.md` — chassis system: the four chassis stats, power-budget authoring,
  limb mounts, Power Draw vs. Strain, elements-as-synergy, visual identity.
- `creature_notes.md`, `battle_notes.md` — raw working notes the above were
  distilled from.

## Project layout

State-machine game loop in `main.rs` → `game.rs`; only one game state is active at
a time, and the UI is a pure view layer that returns intents for the loop to apply.

- `src/data.rs` + `src/data/` — JSON-backed definitions (species, graftware, world,
  settlements, factories, quests, balance) loaded from `assets/data/`.
- `src/model.rs` + `src/model/` — runtime domain model (creature instances, party,
  inventory, rider, world/verdict state, story flags, quests). Engine-agnostic.
- `src/combat.rs` + `src/combat/` — the semi-real-time battle engine and its tests.
- `src/ui.rs` + `src/ui/` — pure view layer (overworld, battle, settlement, bench,
  bestiary, verdict/ledger, procedural creature art); returns `UiAction` intents.
- `src/game.rs` + `src/game/` — top-level loop, state transitions, capture scenes.
- `assets/data/` — all balance/content JSON (regions authored under
  `assets/data/regions/`). **Edit data, not constants.**
