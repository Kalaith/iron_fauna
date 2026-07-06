# IRON FAUNA

A darker, survival-framed 2D creature-collector RPG: catch gentle wild creatures,
graft living weapons onto them, and ride them into battle to hold back a world
whose derelict bio-factories still grow their own monsters. Region by region,
decide whether each factory is destroyed, revived, or claimed — and live with
the world your verdicts add up to.

Built with Rust + Macroquad on `macroquad-toolkit` (WebGL + native Windows).

## Design documents

The design canon lives in this directory and drives implementation:

- `game_design.md` — master GDD: high concept, pillars, creature anatomy, combat contexts, factories, verdict system, duelling, progression.
- `combat.md` — combat system: semi-real-time with Wait/Active pause, 6-slot party budget, rider possession/Boost/hopping, standing orders, called shots.
- `creature.md` — chassis system: the four chassis stats, power budget authoring, limb mounts, Power Draw vs. Strain, elements-as-synergy, visual identity.
- `creature_notes.md`, `battle_notes.md` — raw working notes the above were distilled from.

## Run / test

```powershell
cargo run          # native debug
cargo test         # unit tests
cargo clippy --all-targets --all-features -- -D warnings
.\publish.ps1      # full build + deploy (WebGL + Windows)
```

## Screenshot harness

```powershell
.\scripts\capture_ui.ps1 -Scenes <scene1,scene2>
```

Uses the `IRON_FAUNA_CAPTURE_*` env-var hooks from `macroquad_toolkit::capture`.

## Project layout

- `src/data.rs` + `src/data/` — JSON-backed definitions (species, graftware, world, story) loaded from `assets/data/`.
- `src/model.rs` + `src/model/` — runtime domain model (creature instances, party, inventory, rider, world state). Engine-agnostic.
- `src/combat.rs` + `src/combat/` — semi-real-time battle engine.
- `src/ui.rs` + `src/ui/` — pure view layer; returns `UiAction` intents only.
- `assets/data/` — all balance/content JSON. Edit data, not constants.
