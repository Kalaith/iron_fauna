# IRON FAUNA — Game Design Document

*Draft v0.1 — living document.*

> A 2D creature-collector where you find cute creatures, graft living weapons onto them, and ride them into battle to hold back a world that grows its own monsters — while deciding, region by region, whether the old machines that make them should be destroyed, revived, or claimed.

---

## 1. High concept

A darker, survival-framed 2D creature-collector. The world is thinly populated — humans cling to scattered settlement pockets, with no true cities left — because derelict **bio-factories** still gestate living war-machines that would overrun those pockets if no one held the line. You are a trainer: you catch gentle wild creatures, graft bio-mechanical weapons and armor onto them, and ride them into combat out of necessity. The central irony — and the theme — is that holding the line means doing to innocent creatures the exact thing the old civilization did before it destroyed itself.

- **Genre:** Creature-collector / monster-taming RPG with tactical combat and a persistent, choice-driven world layer.
- **Perspective & style:** 2D, side-view, pixel or hand-crafted 2D art. Scale sold through sprite size and parallax rather than 3D.
- **Tone:** Earnest and mature. Cute-and-dangerous played straight, not for laughs. Melancholy, moral weight, survival tension.
- **Visual identity:** cores are drawn kawaii/chibi — large eyes, soft rounded shapes, no "cool" exceptions — and the war-bodies and graftware wrapped around them are deliberately ugly. Battle content is allowed to get gruesome as that shell is torn apart. See `creature.md` §10.
- **Comparables:** the cute-meets-cruel hook of *Palworld*, played 2D and serious; the modular destructible-loadout depth of *Medabots*; the world-revives-as-you-progress structure of *Terranigma*; the "raid the machine that makes the monsters" loop of *Horizon Zero Dawn*, reimagined biologically.
- **Audience:** Adults who grew up on monster-catchers and want one with real stakes and a conscience.
- **Protagonist:** an orphan whose driving motivation is protecting everyone left in the settlement pockets — no family left to lose, so the stakes are everyone else's. That instinct is also the game's central warning sign: wanting to protect everyone is exactly the impulse that led the Progenitors (§7, §8) to weaponize life in the first place.
- **Scope:** a full RPG, not a vertical slice or demo. Small team. Target platforms: itch.io and Steam.

---

## 2. Design pillars

1. **The creature is the prize, the war-body is disposable.** Combat is about peeling away armor and weapons to reach the vulnerable creature at the center — never about killing it. Violence is aimed at the shell.
2. **Every advantage has a cost paid by something innocent.** Weaponizing creatures keeps people alive but strains and endangers the creatures themselves. The player should always feel the trade.
3. **The world remembers your verdicts.** What you do to each bio-factory permanently reshapes its region — toward death, toward revival, or toward a slow relapse into the old catastrophe.
4. **Survival, not heroism.** The player is doing a compromised, necessary, slightly awful thing. The game respects that ambiguity rather than resolving it into a clean power fantasy.

---

## 3. The creature unit

Every combatant — yours and the enemy's — is built from four layers. This anatomy drives the entire combat system.

- **The Core** — the actual living creature: small, cute, vulnerable. It is the real thing you collect and the **win condition** of every fight. Everything else is grown or grafted around it.
- **The War-Body (grown limbs)** — organic mass gestated around the core: torso, head, legs, arms, tail. Limbs host weapon and armor mounts. Crucially, **limbs regrow as long as the core survives**, so losing a limb is a temporary tactical loss, not a permanent one.
- **Graftware (bio-weapons & armor)** — semi-living weapons and plating grafted onto the limbs: grown ordnance, symbiotic gun-organs, chitin armor, bio-electric or spore armaments. Unlike organic limbs, **graftware does not regrow** — destroyed graftware is lost until repaired or replaced between battles.
- **The Rider (operator)** — the player-aligned pilot, tucked at the rear behind the core. The creature handles body, movement, and natural melee; the rider works the graftware — aiming, firing, triggering abilities. The rider is safe until the core is cracked open, at which point they are exposed and the fight is effectively lost. The rider also has their own progression, separate from any single creature: defeating a bio-factory (§7, §9) grants the rider a permanent upgrade, reflecting the trainer's own growing skill rather than just better gear.

---

## 4. Combat system

### 4.1 The core loop of a fight

1. **Assess** the enemy's loadout — which limbs carry which weapons, where the armor is thickest.
2. **Strip & silence** — target specific limbs to blow off the weapons mounted on them and peel away armor. Because the enemy regrows limbs while its core lives, this is a race, not permanent attrition.
3. **Manage the pool** (see 4.2) — every turn is a choice between firing, regrowing your own lost limbs, and reinforcing your core.
4. **Expose the core** — once the shell is stripped, the core is vulnerable.
5. **Crack it** — cracking the exposed core ends the fight as a **capture** (wild/factory) or a **yield** (duel). Never a kill.

Your own side plays defense simultaneously: protect your limbs and core, decide what to sacrifice, and never let your core crack.

### 4.2 The resource economy — the central tension

A single shared vital pool (**Vigor**) fuels three competing actions:

- **Firing** grafted weapons.
- **Regrowing** lost limbs.
- **Reinforcing / shielding** the core.

The pool regenerates slowly over the fight. Because all three draw from the same source, every turn forces a three-way decision: press the attack, rebuild what you lost, or protect the thing that matters. Over-weaponized creatures can't afford to both fire *and* heal — which ties directly into strain (below).

### 4.3 Strain — bond vs. armament

Graftware stresses a living host. Each unit has a **Strain** threshold determined by the creature's temperament (gentler, cuter creatures often have the highest stat ceilings but the lowest strain tolerance). The more graftware bolted on, and the harder it's pushed, the closer the creature comes to breaking:

- Rising strain degrades performance (accuracy drift, slower regrowth).
- At the threshold, the creature may **reject** a graft mid-fight or go **berserk**, ignoring rider commands.

This is the mechanical expression of Pillar 2: the question every build asks is *how far do I weaponize this innocent thing before it turns on me or falls apart.*

### 4.4 Recovery rules (two layers)

- **Organic limbs** regrow during a fight (costs Vigor, takes turns) as long as the core lives.
- **Graftware** destroyed in a fight is gone until **repaired or replaced between fights** — an economy sink that makes every lost weapon matter.

### 4.5 Turn structure — resolved, see `combat.md`

Combat is **2D side-view with fixed positions and an Atelier-style command menu over a real-time clock** — no movement or range; the player commands one ridden creature a turn at a time (Attack → weapon → target/part, Utility, Reinforce, Regrow, Item, Ride-another, Orders), and per-weapon cooldowns are the delay between turns. A player-selectable pace setting (Wait: the menu auto-opens when you can act; Active: you open it under time pressure) governs tempo. This also brought in the full party/rider structure — a battle loadout spent from a 6-slot field budget (Small 1 / Medium 2 / Large 3), with the rest of the party fighting on simplified Aggressive/Defensive standing orders. See `combat.md` for the complete system.

---

## 5. Three combat contexts

The limb-strip / core-crack model isn't a universal rule that strains believability — it applies specifically to **armed** opponents. Splitting the source of creatures gives the game two distinct combat textures out of one system, plus a third for the duelling loop.

- **Wild subdue.** Wild creatures are unarmed or only naturally armed (claws, a tail-swipe). Capture is a gentle **subdue**: carefully wear the creature down and reach its core without harming it. The cozy, low-cruelty collecting loop.
- **Factory dismantle.** Factory-born war-units are pre-armed and hostile. The full **dismantle** loop applies — silence weapons limb by limb, crack the shell, and capturing the core *frees the creature from the war-body grown around it*. You also salvage the graftware off the wreck.
- **Sanctioned duels (NPC).** Formalized trainer duels against NPC opponents, framed in-fiction as **practice** for factory raids and as a **scarcity-economy mechanism** for redistributing gear. Same combat model; exposing the enemy core triggers a **yield**, not a capture — their creature is unharmed and the loser hands over the staked part. (See §9. Real player-vs-player is explicitly out of scope for now; all duelling is against story NPCs.)

---

## 6. Collection & grafting

- **Catch cores, not creatures-as-shipped.** What you collect is always the core — the real animal — whether subdued in the wild or freed from a factory war-body.
- **Build the war-body.** Back at a settlement (or a bound factory), you outfit a core: grow/assign limbs, then graft weapons and armor onto the mounts.
- **Loadout engineering.** Team-building has two axes — *which core* (temperament, strain tolerance, natural type/ability) and *how it's kitted* (graftware loadout). This is the buildcraft depth layer.
- **Salvage economy.** Weapons detach as salvage when a limb is blown off (yours or the enemy's), so battlefields have gear to fight over, grab, and re-mount.
- **Settlement storage.** Your traveling party is capped at a 6-slot budget (`combat.md` §2.1) — the same budget a battle loadout is built from. Every core caught beyond that stays banked at a settlement until you swap it in, so collecting isn't bottlenecked by what you can carry.

---

## 7. The bio-factories

The engine of the world. Each bio-factory (**Gestarium**) is a derelict gestation-plant left by a collapsed bio-tech civilization (**the Progenitors**). Left running untended, they still do what they were built to do: seed a core in a vat, gestate an armored body around it, and splice living ordnance onto it — producing hostile war-units that spill out and threaten nearby settlements.

- **As dungeons.** Raiding a Gestarium is the game's major dungeon structure — fight through its gestation halls, face progressively nastier war-units, reach its heart. Each factory is its own self-contained map, separate from the overworld, and can be multi-layered (multiple floors/depths to descend through before reaching its heart).
- **As a difficulty ramp.** Older or deeper factories birth higher creature/graft tiers, gating progression naturally.
- **As mystery.** The factories are the central question: who built these, and why grow gentle creatures as disposable gun-platforms?
- **Six, worldwide.** Each of the six Gestaria anchors a distinct region/biome.

---

## 8. World & survival layer

- **Traversal.** The overworld is a set of connected 2D maps, Pokémon-style — settlement pockets and wild regions you move between on foot. Each Gestarium (§7) is reached from the overworld but is its own separate map, distinct from the region it anchors, and can span multiple internal layers.
- **Settlement pockets, no cities.** Humanity survives in small, scattered, fortified pockets. There are no substantial cities — the world never recovered.
- **The overrun threat.** If trainers don't hold the line against factory output, pockets get overrun. This is the pressure that justifies weaponizing creatures at all — it's survival, not sport.
- **The trainer's role.** The player is one of the people doing the necessary, compromised work of keeping the swarm back — collecting, grafting, raiding, and deciding the fate of the machines.
- **The buried truth.** The Progenitors did exactly what the player does — weaponized life to survive some earlier threat — and it consumed them. The world's central moral question is whether the player will end where they ended.

---

## 9. The factory verdict system (Terranigma-inspired)

The factories can't be purely the enemy: they are the player's only source of new cores *and* the only thing that can revive a dead world. Destroying them all ends the player's supply and kills the revival loop; only ever avoiding them means no arc. So the decision lives at **each individual factory**, not as one global policy. Each is a region-defining verdict:

- **Purge.** Shut it down permanently. The region becomes safe but stays **dead** — barren, no more war-units, but no revival and no new cores from there. The graveyard-peace outcome.
- **Reseed.** Restore the factory's benign function. The region **revives**, Terranigma-style — dead ground blooms, water clears, gentle wild cores return, settlements can take root. But a live factory is a loaded gun (see relapse).
- **Bind.** Claim the factory for yourself and use it to grow your own cores — becoming the one who now wields the old civilization's tech, with all the quiet horror that implies.

### 9.1 Revival & relapse

A revived region can **repeat the past**. Prosperity around working factory-tech breeds the same temptation that caused the first collapse. Left live and untended, a reseeded region can **relapse** — someone starts grafting weapons again — and the place you saved becomes a tougher threat later in the game.

- **Relapse is stewardship, not punishment.** Revive-and-abandon is high risk; revive-and-invest (station a watching settlement, spend resources holding the line) lowers relapse risk at an ongoing cost.
- **Relapse is content, not a rug-pull.** When it happens, it arrives as a story beat — a settlement you knew, now militarized, that you must confront. The player feels the tragedy because they remember planting the seed.

### 9.2 Persistent world-state

The world's final state is the **sum of the player's verdicts**: some regions left scarred, safe, and dead; others lush but quietly sliding back toward catastrophe; most somewhere in between and clearly the player's responsibility. World-state persists and is reflected in the settlements, encounters, and duelling rings of each region.

---

## 10. Duelling system (NPC, in-story)

Sanctioned trainer duels are woven into the story and economy as a way to practice dismantle skills and redistribute scarce gear — **all against NPC opponents** for now.

- **Where.** Duelling rings live in settlements and reflect their region's state: a thriving pocket has a healthy, high-stakes ring; a relapsing one has desperate, ugly, high-ante duels.
- **Two lanes.**
  - **Practice / ranked** duels: no gear at risk. Ladder standing, currency, cosmetic rewards. The safe, repeatable loop.
  - **Staked** duels: the gear-wagering loop.
- **The Ante.** Both sides stake something up front — a specific part or an equivalent-value wager the system brokers. Players can only stake what they can afford to lose, and cannot stake the graftware currently keeping their creature standing (**protected-loadout floor**). High-stakes but survivable.
- **Win = yield.** Cracking the NPC's exposed core triggers submission — their creature is unharmed, and they hand over the staked part. The same dismantle skills the player uses in factory raids.
- **Build-wager.** Strain adds a second wager: over-graft to bring more firepower and your own core is likelier to falter or go berserk mid-duel. The pre-match question is both *what do I bet* and *how hard do I push my creature*.

---

## 11. Progression & economy

- **Part flow.** Bio-factories **create** graftware and cores; PvE raids and duels **redistribute** it. A trainer who won't raid can still arm up by out-duelling those who do.
- **Sinks.** Destroyed graftware (permanent until repaired), repair/replacement costs, strain management, and settlement/stewardship upkeep keep gear from inflating.
- **Non-purchasable parts.** Staked/earned parts are never bought or cashed out — this keeps the wagering loop as classic in-game gameplay and avoids real-money-gambling framing. (If this ever heads toward real release, wagering classifications vary by region and warrant a proper legal read — flagged, not resolved.)
- **Meta-progression.** ASSUMPTION: hub-and-spoke — settlements as hubs, factories and wilds as expeditions, with a persistent world-map state. (Structure is an open question — see §12.)

---

## 12. Open questions / to be decided

Design decisions this draft assumed or left open, in rough priority order:

1. ~~**Combat turn structure**~~ — **Resolved:** 2D side-view, semi-real-time-with-pause (player-selectable Wait/Active setting), with a rider-possession party structure. See `combat.md`.
2. ~~**Overall structure**~~ — **Resolved:** Pokémon-style connected 2D overworld maps you move between; each Gestarium is its own separate, potentially multi-layered dungeon map reached from the overworld. See §8.
3. ~~**Number of factories/regions**~~ — **Resolved:** six Gestaria worldwide, each anchoring a distinct region/biome. See §7.
4. ~~**Typing system**~~ — **Resolved:** Element is a build-synergy axis for graftware, not a Pokémon-style effectiveness triangle. See `creature.md` §6.
5. ~~**Roster size & creature design language**~~ — **Resolved:** aim for at least 30 species at launch; traveling party is capped at the same 6-slot budget as a battle loadout, with the rest of your catches kept in settlement storage. Visual language is kawaii/chibi cores contrasted against ugly, gruesome grafted war-bodies. See `creature.md` §9–§10.
6. ~~**Rider progression**~~ — **Resolved:** yes — the rider has their own progression, gaining a permanent upgrade per Gestarium defeated, separate from creature/gear upgrades. See §3.
7. ~~**Naming**~~ — **Resolved:** keeping the working names as final — IRON FAUNA, Gestarium, the Progenitors, Vigor, graftware.
8. ~~**Scope/team/platform**~~ — **Resolved:** full RPG (not a slice/demo), small team, targeting itch.io and Steam. See §1.
9. ~~**Player character & framing**~~ — **Resolved:** the player is an orphan whose personal stake is protecting everyone else in the settlement pockets. See §1.

---

*End of draft v0.1. This is a working document — sections are expected to change as systems are prototyped.*