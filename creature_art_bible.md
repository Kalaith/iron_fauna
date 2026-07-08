# Iron Fauna — Creature Art Bible

**Version:** 1.0  
**Date:** July 2026  
**Purpose:** Define the unified visual language for all creatures, ensuring consistency across procedural generation, UI, combat, bestiary, and narrative moments.  
**Core Philosophy:** "Kawaii Core, Gruesome Graft" — something precious that never asked for this.

---

## 1. Visual Thesis

Every creature embodies a tragic contrast:

> **The Core** is innocent, vulnerable, and adorable — something the player instinctively wants to protect.  
> **The War-Body** is grotesque, ill-fitting, and violent — a living weapon forced onto that innocence.

This duality is the emotional heart of *Iron Fauna*. The cuter the core, the more horrifying and wrong the grafts should feel. Violence and damage are directed at the **shell**, never the core itself, until the final reveal.

The provided reference image (`attachments/image.png`) is the **canonical tone setter** — a small, determined rider beside a massive, organic horror with the core's gentle spirit still visible.

**Key Tone Words:**  
- Cute: Soft, rounded, expressive, protective  
- Grafted: Pulsating, bolted-on, mismatched, tragic, bio-mechanical  
- Battle: Brutal, visceral, visceral but purposeful (shell destruction reveals core)

---

## 2. Creature States

### 2.1 Bare Core (Unarmed)
- **Proportions:** Chibi / super-deformed. Large head, small body, stubby or elegant limbs.
- **Eyes:** Oversized, glossy, highly expressive (sparkling when happy, wide with fear, half-lidded when exhausted). Strong highlights and reflections.
- **Silhouette:** Soft, rounded, approachable. Pastel or gentle natural colors.
- **Personality Read:** Forest spirit / baby animal energy. Instantly sympathetic.
- **Use Cases:** Bestiary "clean" view, settlement bench (pre-graft), post-battle rescue cutscenes, overworld when riding bareback (rare).

### 2.2 War-Body / Grafted
- **Core Visibility:** The cute core must remain partially visible — eyes peeking through armor slits, small face or hands exposed, body outline visible under plating. This heightens tragedy.
- **Grafts:** Visibly forced. Bolts, harnesses, stitched flesh, mismatched textures. Grafts look **stolen and repurposed** (e.g., a biological hand cannon grown from an enemy’s severed limb, with the original creature’s eye or mouth repurposed as the barrel).
- **Movement:** Grafts twitch, pulse, and strain. The small core shows physical effort or discomfort under the weight.
- **Damage Progression:** 
  - Light: Cracked plates, leaking ichor.
  - Heavy: Exposed core sections, dangling broken grafts.
  - Defeat: Dramatic shell fracture → reveal of the intact (or lightly injured) kawaii core.

---

## 3. Inspirations & References

**Primary:**
- *Nausicaä of the Valley of the Wind* — Giant insects (Ohmu especially), beautiful yet terrifying scale, gentle cores beneath armor.
- Studio Ghibli overall (soft character design + nature horror).

**Secondary:**
- *Princess Mononoke* (nature corrupted by violence).
- *Made in Abyss* (cute characters facing body horror).
- *The Last of Us* (organic infection aesthetics, but softer base forms).
- Biomechanical influences: H.R. Giger (organic machinery) filtered through Ghibli warmth.
- Creature-collector games (Pokémon, Digimon) but with tragic rather than empowering transformation.

**Reference Image:** `attachments/image.png` — Use this as the gold standard for core/war-body contrast.

---

## 4. Color & Palette System

### Core (Bare)
- Soft pastels, warm earth tones, gentle gradients.
- High saturation in eyes and cheeks.

### War-Body & Element Synergy
Use element to drive graft palettes while preserving core colors underneath:

- **Bio-electric:** Crackling cyan/blue/white veins, glowing arcs.
- **Plant:** Overgrown greens, flowering tumors, vine restraints (vibrant but sickly).
- **Rock:** Craggy grays/browns, barnacle textures, metallic sheen.
- **Fire:** Molten oranges/reds, smoking vents, glowing cracks.
- **Water:** Slick blues/teals, coral growths, glistening membranes.
- **Poison:** Sickly purples/greens, dripping pustules, bioluminescent toxins.

**General War-Body Palette:** Desaturated base with high-contrast glowing accents. Rust, bruised flesh, chitinous blacks and deep reds.

---

## 5. Chassis & Limb Integration

Grafts must respect the chassis system (`creature.md`):

- **Limb Count** determines mount density and visual complexity.
- **Size** affects scale and strain visibility (tiny cores in huge armor = maximum tragedy).
- **Element** tints graft textures and effects.
- **Power Draw / Strain** visualized through pulsing intensity, leaking fluids, or overheating vents.

**Biological Hand Cannon Example:**
- Massive fleshy barrel grown from stolen enemy limb.
- Core’s original texture still visible on the housing.
- Recoil visibly strains the small rider/core.
- Muzzle glows with elemental energy; spent casings are organic (spent spore pods, bone shards, etc.).

---

## 6. Procedural Generation Guidelines (`ui/creature_art.rs`)

- **Base Silhouette:** Driven by chassis stats (Size, Limb Count, archetype).
- **Core Layer:** Fixed per-species kawaii template with color tinting.
- **Graft Layer:** Modular, element-driven pieces with random offset/rotation for "hastily attached" feel.
- **Strain/Damage Overlays:** Dynamic — more Strain = more leaks, discoloration, twitching.
- **Animation Hooks:** Breathing, eye tracking, graft pulsing, recoil on fire.

---

## 7. Style & Rendering Guidelines

- **Overall Aesthetic:** Painterly 2D with strong ink lines (inspired by the reference image). Supports both hand-drawn assets and procedural output.
- **Lighting:** Dramatic rim lighting to separate core from shell. Volumetric god-rays in factories, bioluminescent glows in battle.
- **Scale:** Emphasize size contrast between rider, core, and war-body.
- **Violence:** Gore and destruction focused exclusively on the shell (ichor, chitin shards, leaking resin). Core blood is minimal and bright.
- **Animation Style:** Fluid but weighty. Small cores show strain; heavy grafts move with inertia.
- **UI Presentation:**
  - Bestiary: Split "Bare" / "Grafted" views + damage states.
  - Bench: Real-time preview of graft application.
  - Combat: Dynamic damage, limb severance, core reveal on defeat.

---

## 8. Worked Examples

**Fox (Fast Scout, Bio-electric)**
- Bare: Tiny orange fox with enormous ears and sparkling eyes.
- Grafted: Railgun arm and back thrusters. Electricity arcs painfully across the body.

**Bear (Heavy Siege, Rock)**
- Bare: Chubby round bear cub.
- Grafted: Massive craggy armor plating and shoulder mortar grown from enemy thorax. Core’s face visible in a helmet-like growth.

**Spider (Utility, Plant)**
- Bare: Adorable fuzzy spider with big shiny eyes.
- Grafted: Multiple tumor-like healing pods and sensor arrays sprouting like cancerous vines.

**Sparrowhawk (Flier, Water)**
- Bare: Small, sleek bird with glossy feathers.
- Grafted: Minimal — back-mounted lightweight cannon and cooling membranes. Emphasizes agility and exposure.

---

## 9. Tone & Narrative Consistency

- **Emotional Goal:** Every time a player sends a creature into battle or sees a graft applied, they should feel the weight of the decision.
- **No Edgy Exceptions:** All cores remain genuinely cute. No "cool" or "badass" bare forms.
- **World Integration:** Grafted creatures look like they belong in the decaying Gestaria bio-factories — beautiful ruins producing beautiful horrors.

---

## 10. Implementation Notes & Deliverables

- **Must-Haves:**
  - Strong core/war-body contrast in every asset.
  - Visible core in grafted state.
  - Element-driven visual language.
  - Support for procedural variation without breaking the "precious core" read.

- **Future Expansions:**
  - Unique legendary/rare variants.
  - Rider + creature synergy visuals (shared palettes or matching harnesses).
  - Verdict/Factory influence on local creature aesthetics.

**Approved By:** Game Design + Art Direction  
**Reference Files:** `creature.md`, `attachments/image.png`, `game_design.md`

---

*This Art Bible supersedes earlier notes. All creature art (procedural or hand-crafted) must adhere to these principles.*