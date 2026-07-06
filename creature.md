# IRON FAUNA — Creature & Chassis System

*Draft v0.1 — distilled from `creature_notes.md`. Companion to `game_design.md`: read §3 (The creature unit) and §4 (Combat system) first — this document builds directly on the Core / War-Body / Graftware / Rider anatomy and the Vigor / Strain resource model defined there. It also resolves two of that document's open questions (§12.4 typing system, §12.5 roster design language).*

---

## 1. Design philosophy: chassis, not movesets

Iron Fauna does not need 150 unique abilities to make its roster feel distinct. Pokémon-style design says "this species is defined by its move-list." That doesn't fit a game whose whole thesis (§3, §4 of `game_design.md`) is that the creature is a small vulnerable core wearing a disposable, engineered shell.

Instead, **a species is a chassis**: a fixed set of physical traits (Power, Size, Speed, Limb Count, Element) that determines what kind of war-body can be grown on it and what graftware it can realistically carry. Two players can catch the same species and build very different war-bodies on it, but no species can become "just a worse version" of another, because every trait is a trade-off drawn from the same budget.

> Players won't collect a Fox because it has Quick Attack. They'll collect it because it's the fastest low-Power chassis in the game, perfect for a hit-and-run electrical build.

This is what keeps a fast scout, a heavy siege beast, a strain-limited support, and an all-rounder simultaneously relevant late-game: they fill different engineering roles instead of sitting on one linear power curve.

---

## 2. The four chassis stats

Every species is defined by four physical stats, rolled at species-design time (see §3, Power Budget). These sit *underneath* the anatomy from `game_design.md` §3 — they describe the Core's biology, which in turn shapes what War-Body and Graftware can be built on it.

### 2.1 Power — biological capacity

Power is **not** a combat stat you spend mid-fight — that's what Vigor (`game_design.md` §4.2) is for. Power is the ceiling that determines how large and capable that whole system gets:

- **Power Capacity** — the maximum total weight of graftware (§5) the creature can carry before it strains.
- **Vigor pool size** — how much Vigor the creature has to spend across firing / regrowing / reinforcing each fight.
- **Strain threshold** — how much abuse (§5, and `game_design.md` §4.3) the creature can take before it rejects a graft or goes berserk.
- **Limb regrowth speed** — how fast severed limbs regrow while the core survives.
- **Minimum Power to wield heavy graftware** — some weapon/armor tiers simply require a Power floor, independent of available mount slots.

High Power does not automatically mean "better" — it typically comes bundled with worse Size and Speed (§3), which is the whole point of the budget.

### 2.2 Size — the trait that touches everything

Size shouldn't just be a bigger HP number. It should shift the creature's entire playstyle:

| | Small | Large |
|---|---|---|
| To-hit | Hard to hit | Easy to hit |
| Speed | Faster | Slower |
| Weapon weight ceiling | Light weapons only | Can mount heavy weapons |
| Vigor pool | Lower | Higher |
| Carrying / mount capacity | Lower | Higher |
| Stealth / positioning | Easy to hide, fits anywhere | Can't fit everywhere |

### 2.3 Speed — more than a turn-order stat

Assuming turn-based tactical combat (`game_design.md` §4.5, still unconfirmed):

- **Fast creatures** gain: acting first, higher dodge, easier flanking, lower weapon recoil penalties, easier disengage/retreat.
- **Slow creatures** gain: positional stability, better accuracy, a heavy-weapon damage bonus, knockback resistance.

Neither end is strictly better — fast creatures out-position and pick fights; slow creatures dictate them once engaged.

### 2.4 Limb Count — the tactical skeleton

Limb count sets how many mount points exist for graftware (§4) and how much War-Body has to regrow if the creature gets stripped mid-fight (`game_design.md` §3, §4.4). Four rough archetypes:

| Limbs | Archetype | Reads as |
|---|---|---|
| 2 | **Flier** | No arm mounts at all — trades graftware capacity entirely for natural flight. In combat this plays as a mobility/evasion specialist (§7); flight has no special in-fight terrain rules for now (§9) — its bigger role is overworld traversal, letting the player move faster across the map (`game_design.md` §8). |
| 4 | **Standard** | The balanced baseline; most starter-tier chassis live here. |
| 6 | **Utility** | Extra mount points for support/utility graftware at the cost of a bigger profile to defend. |
| 8 | **Heavy** | Maximum mount count and carrying capacity; high baseline Strain and poor repositioning. |

More limbs is not a strictly-better upgrade: each additional limb is more War-Body to protect and regrow (draws Vigor, per `game_design.md` §4.4), and mount *slots* still have to be paid for out of Power (§4) — a Centipede-class chassis with weak Power ends up "all mounts, nothing to power them."

---

## 3. Power Budget (species authoring)

Every species is designed against a fixed **Power Budget** (placeholder value: 100 points) spent across the traits above. This is a *design-time* authoring tool for building species that are automatically balanced against each other — it is separate from the player-facing **Power Capacity** a creature exposes during graftware loadout (§5), which players see and spend directly.

Placeholder cost table — treat every number here as a first pass for prototyping, not a final balance target:

| Trait | Cost |
|---|---|
| Size: Small | 0 |
| Size: Medium | 15 |
| Size: Large | 35 |
| Size: Huge | 60 |
| Extra limb pair | 10 |
| Speed: Fast | 25 |
| Speed: Very Fast | 45 |
| Heavy Armor (innate) | 30 |
| High Vigor (innate) | 30 |
| Natural Flight | 35 |

Whatever budget isn't spent on the traits above rolls into the species' base Power score and secondary stats (agility, base regen, etc.).

**Worked examples, budget = 100:**

- **Rabbit-type** — Small, Very Fast, 4 limbs, remainder into agility. *Result:* hard to hit, only two light mounts, low Vigor pool.
- **Crab-type** — Medium, 6 limbs, Heavy Armor, Slow. *Result:* four usable weapon mounts, high defense, easy target.
- **Elephant-type** — Huge, 8 limbs, Very Slow, remainder into Vigor. *Result:* walking tank, huge graftware capacity, can barely reposition.

---

## 4. Limb mounts & graftware capacity

Weapon and armor mounts are **not** free per limb — each limb has a fixed carrying capacity, and how many *usable* mounts a species has depends on Limb Count *and* Size/Power together. Two chassis can share a limb count and still carry very different loadouts:

| Chassis example | Limbs | Mounts |
|---|---|---|
| Rabbit-type (small, standard) | 4 | 2 Light |
| Wolf-type (medium, standard) | 4 | 4 Medium |
| Crab-type (medium, utility) | 6 | 6 Light + 2 Heavy |
| Elephant-type (huge, heavy) | 8 | 4 Heavy + 4 Medium |

A Rabbit isn't simply "a worse Wolf" — it's a different mount profile (few, light, high-agility mounts vs. more, heavier, slower ones), which is what keeps low-limb-count chassis relevant into the late game.

---

## 5. Power Draw — how this plugs into Strain

This is the piece that ties the chassis system directly into `game_design.md` §4.3 (Strain).

Each piece of equipped graftware has a **Power Draw** cost. A creature's **Power Capacity** (from its Power stat, §2.1) is the budget that draw is checked against — this is player-facing: the loadout/equip screen shows remaining Power Capacity live as graftware is added, so a player can see exactly how much headroom they have left before a pick starts contributing to Strain.

```
Creature Power Capacity: 120

Cannon        60
Machine Gun   20
Shield        30
Healing Pod   25
---------------------
Total         135   (exceeds capacity by 15)
```

The loadout above is legal to *equip* — mount slots allow it — but it exceeds the creature's Power Capacity. That overdraw feeds straight into Strain (`game_design.md` §4.3):

- Passive Strain rises over the course of the fight.
- Accuracy drifts and limb regrowth slows.
- Vigor regeneration drops and movement is penalized.
- At the Strain threshold: graft rejection or berserk, same as any other strain source.

This is the answer to "why doesn't six weapon mounts just mean six powerful weapons": mounts are a *capacity* limit, Power Draw is a *sustainability* limit, and only chassis with both mount slots *and* the Power to back them up can run a maximal loadout without punishing themselves.

---

## 6. Elements: graftware synergy, not type matchups

This resolves `game_design.md` §12.4. Elements are **not** a Pokémon-style effectiveness triangle (no "supereffective," no rock-paper-scissors combat resolution). Instead, a creature's Element is an innate trait — like Power or Size — that biases which *graftware* performs best on it. Elements shape builds; they don't gate or decide fights on their own.

| Element | Graftware synergy |
|---|---|
| Bio-electric | Electrical weapons |
| Plant | Healing / regeneration graftware |
| Rock | Heavy armor |
| Fire | Heat-based weapons |
| Water | Cooling / heat-dissipation systems |
| Poison | Toxin weapons |

A Fire-element chassis can still carry a shield or a cooling unit — it'll just get less out of it than a Water-element chassis would. Element is a build lean, not a hard restriction.

The very first draft of `creature_notes.md` also floated a taxonomic **Type** (Mammal, Reptile, Arthropod, Avian...) alongside Element. That's dropped: Element is the only classification axis a species carries. Species flavor/taxonomy, if it matters at all, is art direction — not a second overlapping stat.

---

## 7. Chassis archetypes — worked species

Full stat-block format for species design, covering each Limb Count archetype from §2.4:

**Fox** — *fast scout*
- Power 40 · Speed 95 · Size Small · Limbs 4 (Standard) · Element Bio-electric
- 2 Light mounts. Plays as a hit-and-run skirmisher: dances around heavier chassis, can't out-tank a sustained fight, punishes overcommitted slow creatures.

**Bear** — *heavy siege*
- Power 90 · Speed 40 · Size Large · Limbs 6 (Utility) · Element Rock
- 4 Medium + 2 Heavy mounts. Hauls a cannon, a shield, and a mortar simultaneously without exceeding Power Capacity — a slow-moving fortress that dictates the fight once it closes.

**Spider** — *utility support*
- Power 55 · Speed 60 · Size Medium · Limbs 6 (Utility) · Element Plant
- 6 Light mounts. Can *mount* six utility devices (healing pods, sensors, buffs) but lacks the raw Power to fire more than two or three continuously without straining — a support chassis whose skill ceiling is loadout discipline, not raw stats.

**Sparrowhawk** — *flier scout*
- Power 30 · Speed 85 · Size Small · Limbs 2 (Flier) · Element Water
- No arm mounts at all; relies entirely on natural flight and evasion plus one back-mounted Light graft. Reads as the purest "avoid damage entirely" playstyle in the roster.

Each of these remains useful at any point in the game because it occupies a distinct engineering niche (scout / siege / support / flier) rather than a rung on a shared power ladder — directly reinforcing `game_design.md`'s framing of creatures as disposable-shell, precious-core units rather than a Pokédex of numbers-go-up monsters.

---

## 8. Why this instead of a bigger movepool

The alternative this document is explicitly rejecting is designing ~150 creatures around ~150 unique abilities. That approach front-loads enormous content cost and still tends to collapse into a linear power curve (later creatures are just numerically better). The chassis model instead guarantees permanent niches by construction: a Fox can never become "worse" once you catch a Bear, because they are not competing on the same axis. This also dovetails with `game_design.md` §11 (Progression & economy) — since graftware, not species, is where raw power actually scales, a low-Power, high-Speed chassis stays mechanically relevant into the endgame as a chassis for late-game graftware, not just an early-game placeholder.

---

## 9. Resolved vs. still open

**Resolved:**

- **Numbers stay placeholders for now.** Power Budget total, trait costs, mount counts, and Power Draw values throughout §3–§5 are deliberately unbalanced pending a dedicated prototyping/playtesting pass — that's an accepted gap, not a blocker for continued design work.
- **Power Capacity is player-facing** (§5) — shown live on the loadout/equip screen so players can see remaining capacity and balance it against the graftware they're fitting. (The species-authoring Power *Budget* in §3 stays a designer-side tool — a separate thing from this.)
- **Type is dropped.** Element (§6) is the only classification axis a species carries.
- **Flight is out of scope for the current battle system.** No special in-fight terrain/positioning rules for flying creatures at launch (`game_design.md` §4.1's loop applies to them the same as anyone else). Flight's real payoff is overworld traversal — a Flier-trait creature lets the player move faster across the map (`game_design.md` §8).
- **The Element list is final for now.** The six in §6 (Bio-electric, Plant, Rock, Fire, Water, Poison) are the launch set, not just a starting sketch.
- **Roster size at launch: aim for at least 30 species.** The chassis system (§1–§8) is what keeps that roster differentiated regardless of count; the player's traveling party is capped at the same 6-slot budget as a battle loadout (`combat.md` §2.1) — everything caught beyond that is kept in settlement storage, not carried.
- **Visual design language: kawaii core, gruesome war-body.** See §10.

**Still open:**

None remain.

---

## 10. Visual identity: kawaii core, gruesome war-body

The roster's art direction is a deliberate contrast, not a uniform style:

- **The Core, unarmed.** Super kawaii, chibi proportions, large eyes, soft rounded shapes. Every species reads as something you want to protect on sight — no "cool" or "edgy" outliers. This is the whole roster's baseline, before any war-body or graftware is added.
- **Grafted and armed.** The war-body and its graftware break that cuteness on purpose: ugly, ill-fitting, visibly bolted-on plating and weapons wrapped around something that was never built to carry them.
- **In battle.** Content is allowed to get gruesome. The violence is aimed at the shell (`game_design.md` §3, §4.1) — the visual horror belongs to the war-body being torn apart, not to the creature underneath. The uglier and more violent the shell's destruction reads, the harder the reveal of the intact, precious core at the center should land.

This isn't a tonal inconsistency — it's the game's central irony (`game_design.md` §1) made visual. The more adorable the core, the more wrong the machinery wrapped around it should feel, and the more it should sting to watch it fight.

---

*End of draft v0.1. Numbers throughout are illustrative placeholders pending a dedicated balance pass — see §9.*
