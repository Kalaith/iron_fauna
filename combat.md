# IRON FAUNA — Combat System

*Draft v0.2 — distilled from `battle_notes.md`. Companion to `game_design.md` (§3 anatomy, §4 combat system, §5 combat contexts) and `creature.md` (chassis stats, limb mounts, Strain/Power Draw). **Combat is 2D side-view with fixed positions and an Atelier-style command menu over a real-time clock:** you directly command one ridden creature a turn at a time while the rest of the party fights on standing orders. This supersedes the earlier "semi-real-time ATB with free movement" direction (v0.1) — there is no positioning or movement, and commands are issued from a menu, not action-game keys.*

---

## 1. Format: real-time clock, menu commands, fixed positions

Creatures hold **fixed positions** — there is no movement or range on the battlefield; every combatant can always reach every foe. What differs between weapons is damage, cost, and **per-weapon cooldown**, not distance.

The player commands exactly one creature — the ridden one — through a **command menu** (Attack → weapon → target/part, Utility, Reinforce, Regrow, Item, Ride-another, Orders, Wait). Issuing a command executes it immediately; the weapon's cooldown is the delay before that weapon can be used again. Everything not ridden fights autonomously on standing orders (§4). The real-time clock only advances while the menu is closed — so a turn is: *menu opens → you pick a command → it resolves → the clock runs until you can act again.*

A battle-pacing setting controls when the menu opens:

- **Wait (default).** Whenever the ridden creature has an action available, the field pauses and the command menu opens for you. Pick a command and the clock resumes until the next action comes off cooldown. Choosing **Wait** in the menu lets the clock run without acting; press Enter/Space anytime to summon the menu again.
- **Active.** The clock never auto-opens the menu — you press Enter/Space to open it when you want to act, so you're deciding under time pressure. A manual full-pause hotkey (`P`) exists in both modes.

This is one system with a difficulty/tempo lever, not two combat modes — Wild subdue, Factory dismantle, and Sanctioned duels (`game_design.md` §5) all run on it identically; only the fiction and win-condition differ.

---

## 2. The party & the rider

The core reframe from `battle_notes.md`: the constraint isn't "how many creatures can you field," it's "there's only one of you." You bring a party and they all fight — but you are a single roving force-multiplier the party has to share, not several independently piloted units.

### 2.1 Party slots

A battle loadout is built from a fixed **6-slot budget**, spent on creatures by Size class (`creature.md` §2.2):

| Size | Slot cost |
|---|---|
| Small | 1 |
| Medium | 2 |
| Large | 3 |
| Huge | 4 |

That budget is flexible by design — the same 6 slots can produce very different parties:

- 6 Small creatures — a swarm of fast, fragile skirmishers.
- 2 Large creatures — a pair of walking fortresses.
- 1 Small + 1 Medium + 1 Large — a balanced mixed squad.
- 1 Huge + 1 Medium — a single walking fortress with one support unit backing it up.

You can own a larger overall roster of caught creatures (the collection layer, `game_design.md` §6), but your traveling party — and every individual fight — is fielded from this same 6-slot budget. Anything caught beyond that stays in settlement storage (`game_design.md` §8) until you swap it in. This also resolves the 2D-lane readability concern raised in `battle_notes.md` without needing a separate "active field cap": Large creatures are the ones with the biggest, busiest sprites, and their slot cost naturally caps how many can be on screen at once (max 2), while cheap, simple Small sprites can crowd the field in larger numbers without it becoming unreadable.

### 2.2 The rider

At any moment, exactly one fielded creature is *ridden* (possessed) and under your direct control. Every other fielded creature fights autonomously on Standing Orders (§4). A party fight is therefore a semi-autonomous pack with one spearhead wherever you currently are.

**Presentation note:** the ridden creature is shown with the rider sprite perched on top of its head — a simple, always-visible read on which creature currently has your direct control. This is a legibility convention for the 2D sprite, distinct from the interior "protected until the core cracks" safety fiction in `game_design.md` §3, which describes the rider's actual in-fiction position tucked behind the core.

---

## 3. Riding & the Boost

Riding a creature grants two things an autonomous (AI-run) creature cannot have:

- **Manual control** — active dodging, positioning, and **called shots**.
- **The Boost** — the ridden creature's strongest graft unlocks, and its Vigor regenerates faster (`game_design.md` §4.2).

### 3.1 Called shots

Called shots are chosen from the **Strike** submenu after picking a weapon: the target's strikeable spots are listed — center mass, each mounted graft (with its remaining HP), and each bare limb — and you pick one. `<-`/`->` switch which enemy is in the crosshair. This is the mechanical tool for the "Strip & silence" step of the core combat loop (`game_design.md` §4.1) and plugs directly into `creature.md` §4's mount system — you're choosing *which* mount to blow off, not just doing generic damage. Autonomous creatures under Standing Orders (§4) attack whatever's nearest/valid and cannot make called shots — if a fight needs precision, stripping one specific mount off one specific enemy, you have to be riding a creature that can reach it. Only the rider can do surgery.

### 3.2 The Boost is creature- and weapon-dependent

The Boost isn't one universal effect — what it does depends on **which creature** you're riding and **which graftware** it currently has equipped. A creature boosted while carrying an electrical weapon might chain damage between enemies; the same creature boosted with a healing pod equipped might trigger a burst heal instead. This keeps loadout choice (`game_design.md` §6) meaningful turn-to-turn, not just as a pre-fight stat check — the same species can produce a noticeably different Boost depending on how it's kitted.

---

## 4. Standing Orders — kept deliberately simple

Non-ridden creatures act on a player-set stance rather than freelance AI. The baseline system is intentionally minimal:

- **Aggressive** — prioritize attacking; spend Vigor freely on firing.
- **Defensive** — prioritize guarding and mitigation; conserve Vigor, favor reinforcing the core over pressing attacks.
- **Cooldowns, not scripting.** Every creature's actions — ridden or autonomous — are gated by per-action cooldowns rather than a resource queue or scripted rotation. An Aggressive creature simply fires whatever's off cooldown at the nearest valid target; a Defensive one holds its cooldowns for reinforcing/guarding unless directly threatened.

Orders are set before a fight (loadout/prep screen) and can be flipped mid-fight per creature — itself a decision point, so it auto-pauses the field in Wait mode (§1) like any other command.

**Design rule:** orders must be explicit and player-set, never a hidden "smart AI." The entire party-of-autonomous-units idea only works if a bad outcome reads as *"I had that creature on the wrong stance"* — fixable next time — rather than *"the AI played badly,"* which isn't. This is the single biggest risk called out in `battle_notes.md`; keeping the system to two legible stances plus cooldowns (rather than a deeper gambit-scripting layer) is the mitigation. A richer order system (targeting priorities, conditional triggers) is a possible post-MVP layer, not the baseline.

---

## 5. Rider-hopping

Switching which creature you're riding is a real, costed action, not a menu toggle:

- **Time.** Choosing "Ride another" from the menu picks the new mount; the hop then takes a short transit time on the clock before you have control of it.
- **Exposure.** During that transit the rider is between mounts and not shielded behind any creature's core. Both the creature being left and the one being boarded are AI-controlled (on their last-set Standing Orders) until the hop lands. If the destination is cracked mid-transit, the rider lands exposed and must hop again.

This makes every hop a genuine tactical question — *where am I needed most right now* — weighed against real risk, not a free camera swap. Hopping is also the primary tool for the Strain and core-crack tensions below.

---

## 6. Strain & the incentive to rotate

This connects directly to `creature.md` §5 (Power Draw vs. Power Capacity) and `game_design.md` §4.3 (Strain).

The Boost pushes the ridden creature harder, so **whichever creature you're currently riding accumulates Strain faster than it would running on its own orders.** Stay mounted on one favorite creature for an entire fight and it will over-strain — rejecting a graft or going berserk, same as any other Strain source. Spreading your presence across the party isn't just for positioning anymore; it lets each creature you dismount "cool off." Rider-hopping is simultaneously a positioning tool and a Strain-management tool — one mechanic, two reasons to use it.

---

## 7. The core-crack danger state

Cracking an exposed core (`game_design.md` §4.1, step 5) ends that creature's participation in the fight — a capture, a freed core, or a yield depending on context (§8). In a multi-creature party battle, this downs one combatant, not the whole encounter — unless it was the last one standing on that side. Riding raises the stakes on top of that base rule:

The ridden creature is your highest-value unit on the field — boosted, and carrying you — so enemies will focus it. If the creature you're riding has its core cracked, that creature is immediately out of the fight and you (the rider) are left exposed on the field until you hop (§5) to another still-standing fielded creature. Riding makes a creature stronger and paints a target on it at the same time, which is exactly the tension that should push you to bail off a creature that's about to fall — even mid-plan — rather than ride it down.

### 7.1 Losing the fight

If every creature in your fielded party has its core cracked, the encounter ends there and **you flee and escape.** This is a soft loss, not a game over — consistent with the "never a kill" framing (`game_design.md` §4.1), losing a fight costs you the engagement and whatever you'd have gained from winning it, not your creatures, your progress, or the run itself.

---

## 8. Applying this across the three combat contexts

Same engine underneath all three from `game_design.md` §5 — only the fiction and end-state change:

- **Wild subdue** — the wild creature is unarmed or naturally armed only; there are no enemy graftware mounts to call-shot, so the loop is closer to careful positioning and wearing down Vigor than a full dismantle.
- **Factory dismantle** — the full loop applies: called shots strip weapon mounts limb by limb, standing orders hold the line on the creatures you're not riding, and cracking the core frees it from the war-body.
- **Sanctioned duels** — identical system against an NPC opponent; cracking their exposed core resolves as a yield instead of a capture.

---

## 9. Resolved vs. still open

**Resolved by this document:**

- Combat format: 2D side-view, fixed positions (no movement/range), real-time clock with an Atelier-style command menu; one ridden creature commanded a turn at a time (§1).
- Pacing control: player-selectable Wait/Active setting governing when the command menu opens (§1).
- Party structure: 6-slot budget spent by Size (Small 1 / Medium 2 / Large 3), rider-possession + Standing Orders, not full simultaneous multi-unit control (§2).
- Standing Orders scope: kept to Aggressive/Defensive stances plus per-action cooldowns for the MVP, not a deeper gambit-scripting system (§4).
- Called-shot input: chosen from the Strike submenu (center mass / each graft / each limb), with `<-`/`->` to switch foe (§3.1).
- The Boost: not fixed — depends on the ridden creature's species *and* its currently equipped graftware (§3.2).
- Player-side core-crack consequences: an individual core-crack downs that one creature; losing every fielded creature ends the encounter in a flee/escape, not a game over (§7.1).
- Huge-class slot cost: 4 slots (§2.1) — Huge is player-fieldable, just expensive.
- Field budget: confirmed at 6 slots — this is also the traveling-party cap, not just a battle-loadout number (§2.1). Exact per-size costs still get final tuning during production, same as any balance number.

No open items remain from `battle_notes.md`'s original list.

---

*End of draft v0.1. Assumes 2D side-view real-time-with-pause per this session's direction; party slot costs are placeholders pending prototyping — see §9.*
