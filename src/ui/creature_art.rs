//! Procedural war-body art: a deterministic 2D creature sprite generated from
//! its species chassis, with graftware bolted on to show the before/after of
//! grafting (`creature.md`, `game_design.md` §3).
//!
//! Modelled on `toybox`'s compositional toy renderers — every creature is a
//! stack of primitive shapes at scaled offsets, seeded from the species id so
//! each chassis looks distinct but never changes between frames. Element sets
//! the palette; size/archetype/temperament reshape the silhouette; the limb
//! list decides which regions can sprout grafts.

use crate::data::graftware::GraftKind;
use crate::data::species::{Element, LimbArchetype, LimbRegion, SizeClass, SpeciesDef};
use crate::data::GameData;
use crate::model::creature::CreatureInstance;
use crate::state::GameSession;
use crate::ui::element_color;
use crate::util::Rng;
use macroquad::prelude::*;

/// One graft reduced to just what the sprite needs to bolt it on.
pub struct GraftVisual {
    pub region: LimbRegion,
    pub kind: GraftKind,
    pub element: Option<Element>,
    pub broken: bool,
}

/// Build the visual graft list for a creature's current loadout — the "after"
/// half of the before/after. An empty result draws the bare chassis.
pub fn grafts_for_creature(
    data: &GameData,
    session: &GameSession,
    creature: &CreatureInstance,
) -> Vec<GraftVisual> {
    let species = creature.species(data);
    creature
        .loadout
        .iter()
        .filter_map(|m| {
            let item = session.profile.inventory.item(m.item_id)?;
            let def = data.graftware.get(&item.def_id)?;
            let limb = species.limb(&m.limb_id)?;
            Some(GraftVisual {
                region: limb.region,
                kind: def.kind,
                element: def.element,
                broken: !item.is_usable(),
            })
        })
        .collect()
}

/// A representative "fully outfitted" loadout for a chassis — one graft of each
/// kind spread across the regions it can actually mount. Used for the bestiary's
/// before/after preview where there is no real instance loadout to draw.
pub fn sample_grafts(species: &SpeciesDef) -> Vec<GraftVisual> {
    let kinds = [GraftKind::Weapon, GraftKind::Armor, GraftKind::Utility];
    let mut regions: Vec<LimbRegion> = Vec::new();
    for limb in &species.limbs {
        if !limb.mounts.is_empty() && !regions.contains(&limb.region) {
            regions.push(limb.region);
        }
    }
    regions
        .into_iter()
        .take(4)
        .enumerate()
        .map(|(i, region)| GraftVisual {
            region,
            kind: kinds[i % kinds.len()],
            element: Some(species.element),
            broken: false,
        })
        .collect()
}

/// How a war-body is coloured — full colour, a flat silhouette, or the grey of
/// a cracked core.
#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Normal,
    Silhouette,
    Downed,
}

/// Draw a fully-realised war-body centred at `(cx, cy)`, `u` = base unit (about
/// half the body height in px). Grafts overlay at their limb regions.
pub fn draw_war_body(cx: f32, cy: f32, u: f32, species: &SpeciesDef, grafts: &[GraftVisual]) {
    render(cx, cy, u, species, grafts, Mode::Normal);
}

/// Draw a defeated war-body: greyed out, grafts hanging dead. Used in battle
/// when a core is cracked (`game_design.md` §5).
pub fn draw_downed(cx: f32, cy: f32, u: f32, species: &SpeciesDef, grafts: &[GraftVisual]) {
    render(cx, cy, u, species, grafts, Mode::Downed);
}

/// Draw the chassis as a flat shadow — the uncaught-in-bestiary silhouette.
pub fn draw_silhouette(cx: f32, cy: f32, u: f32, species: &SpeciesDef) {
    render(cx, cy, u, species, &[], Mode::Silhouette);
}

/// Screen point of a limb region's anchor, so callers can pin combat markers
/// (severed stumps, aim reticles) exactly where that region's grafts render.
pub fn region_point(cx: f32, cy: f32, u: f32, region: LimbRegion) -> Vec2 {
    let (offset, _dir) = region_anchor(region, u);
    vec2(cx, cy) + offset
}

fn brighten(c: Color, a: f32) -> Color {
    Color::new(
        (c.r + a).min(1.0),
        (c.g + a).min(1.0),
        (c.b + a).min(1.0),
        c.a,
    )
}

fn darken(c: Color, a: f32) -> Color {
    Color::new(
        (c.r - a).max(0.0),
        (c.g - a).max(0.0),
        (c.b - a).max(0.0),
        c.a,
    )
}

fn mix(a: Color, b: Color, t: f32) -> Color {
    Color::new(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
        a.a,
    )
}

/// Stable per-species seed (FNV-1a over the id) so jitter never flickers.
fn species_seed(species: &SpeciesDef) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for b in species.id.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// Torso width multiplier — heavies are broad, fliers are lean.
fn width_factor(species: &SpeciesDef) -> f32 {
    let arch = match species.archetype {
        LimbArchetype::Heavy => 1.28,
        LimbArchetype::Flier => 0.82,
        LimbArchetype::Utility => 0.9,
        LimbArchetype::Standard => 1.0,
    };
    let size = match species.size {
        SizeClass::Small => 0.9,
        SizeClass::Medium => 1.0,
        SizeClass::Large => 1.08,
        SizeClass::Huge => 1.16,
    };
    arch * size
}

struct Palette {
    base: Color,
    belly: Color,
    line: Color,
    silhouette: bool,
}

fn render(cx: f32, cy: f32, u: f32, species: &SpeciesDef, grafts: &[GraftVisual], mode: Mode) {
    let mut rng = Rng::new(species_seed(species));
    let flat = mode == Mode::Silhouette;
    let base = match mode {
        Mode::Silhouette => Color::new(0.16, 0.17, 0.21, 1.0),
        Mode::Downed => Color::new(0.30, 0.30, 0.34, 1.0),
        Mode::Normal => element_color(species.element),
    };
    let pal = Palette {
        base,
        belly: brighten(base, 0.14),
        line: if flat {
            Color::new(0.10, 0.11, 0.14, 1.0)
        } else {
            Color::new(0.08, 0.07, 0.09, 0.9)
        },
        silhouette: flat,
    };
    let w = width_factor(species);
    let has_tail = species.limbs.iter().any(|l| l.region == LimbRegion::Tail);
    let has_arms = species
        .limbs
        .iter()
        .any(|l| matches!(l.region, LimbRegion::ArmLeft | LimbRegion::ArmRight));

    // Ground shadow.
    if !flat {
        draw_ellipse(
            cx,
            cy + 0.92 * u,
            0.72 * u * w,
            0.16 * u,
            0.0,
            Color::new(0.0, 0.0, 0.0, 0.28),
        );
    }

    if species.natural_flight {
        draw_wings(cx, cy, u, &pal, &mut rng);
    }
    if has_tail {
        draw_tail(cx, cy, u, &pal, &mut rng);
    }
    draw_legs(cx, cy, u, w, species, &pal);
    draw_torso(cx, cy, u, w, &pal);
    if has_arms {
        draw_arms(cx, cy, u, w, &pal);
    }
    draw_head(cx, cy, u, species, &pal, &mut rng);

    // Grafts read as full-colour hardware — only on a living, coloured body.
    if mode == Mode::Normal {
        draw_grafts(cx, cy, u, grafts);
    }
}

fn draw_torso(cx: f32, cy: f32, u: f32, w: f32, pal: &Palette) {
    let ry = 0.52 * u;
    let rx = 0.6 * u * w;
    let ty = cy + 0.14 * u;
    draw_ellipse(cx, ty, rx, ry, 0.0, pal.base);
    draw_ellipse_lines(cx, ty, rx, ry, 0.0, 2.0, pal.line);
    if !pal.silhouette {
        // Belly patch.
        draw_ellipse(cx, ty + 0.16 * u, rx * 0.55, ry * 0.62, 0.0, pal.belly);
    }
}

fn draw_legs(cx: f32, cy: f32, u: f32, w: f32, species: &SpeciesDef, pal: &Palette) {
    let (count, thick) = match species.archetype {
        LimbArchetype::Heavy => (4, 0.20),
        LimbArchetype::Flier => (2, 0.10),
        LimbArchetype::Utility => (2, 0.13),
        LimbArchetype::Standard => (4, 0.15),
    };
    let top = cy + 0.5 * u;
    let foot = cy + 0.9 * u;
    let spread = 0.42 * u * w;
    let leg_color = darken(pal.base, 0.1);
    // Back pair first (inset + darker), then the front pair over the torso.
    if count == 4 {
        for s in [-1.0_f32, 1.0] {
            leg(
                cx + s * spread * 0.55,
                top - 0.05 * u,
                foot,
                thick * u * 0.9,
                darken(leg_color, 0.06),
                pal,
            );
        }
    }
    let pairs = if count >= 2 {
        [-1.0_f32, 1.0]
    } else {
        [0.0, 0.0]
    };
    for s in pairs {
        leg(cx + s * spread, top, foot, thick * u, leg_color, pal);
    }
}

fn leg(x: f32, top: f32, foot: f32, half_w: f32, color: Color, pal: &Palette) {
    draw_rectangle(x - half_w, top, half_w * 2.0, foot - top, color);
    draw_circle(x, foot, half_w * 1.1, color);
    if !pal.silhouette {
        draw_rectangle_lines(x - half_w, top, half_w * 2.0, foot - top, 1.5, pal.line);
    }
}

fn draw_arms(cx: f32, cy: f32, u: f32, w: f32, pal: &Palette) {
    let color = darken(pal.base, 0.08);
    let sx = 0.58 * u * w;
    for s in [-1.0_f32, 1.0] {
        let x = cx + s * sx;
        draw_ellipse(x, cy + 0.12 * u, 0.14 * u, 0.3 * u, s * 0.2, color);
        draw_ellipse_lines(x, cy + 0.12 * u, 0.14 * u, 0.3 * u, s * 0.2, 1.5, pal.line);
        draw_circle(x + s * 0.02 * u, cy + 0.4 * u, 0.11 * u, color);
    }
}

fn draw_tail(cx: f32, cy: f32, u: f32, pal: &Palette, rng: &mut Rng) {
    let dir = if rng.chance(0.5) { 1.0 } else { -1.0 };
    let color = darken(pal.base, 0.05);
    let mut px = cx + dir * 0.5 * u;
    let mut py = cy + 0.32 * u;
    for i in 0..4 {
        let r = (0.2 - i as f32 * 0.04) * u;
        draw_circle(px, py, r.max(0.04 * u), color);
        px += dir * 0.22 * u;
        py += (0.06 - i as f32 * 0.05) * u;
    }
}

fn draw_wings(cx: f32, cy: f32, u: f32, pal: &Palette, _rng: &mut Rng) {
    let membrane = if pal.silhouette {
        pal.base
    } else {
        Color::new(pal.belly.r, pal.belly.g, pal.belly.b, 0.72)
    };
    for s in [-1.0_f32, 1.0] {
        let root = vec2(cx + s * 0.2 * u, cy - 0.1 * u);
        let tip = vec2(cx + s * 1.05 * u, cy - 0.62 * u);
        let low = vec2(cx + s * 0.75 * u, cy + 0.12 * u);
        draw_triangle(root, tip, low, membrane);
        draw_triangle_lines(root, tip, low, 1.5, pal.line);
    }
}

fn draw_triangle_lines(a: Vec2, b: Vec2, c: Vec2, thick: f32, color: Color) {
    draw_line(a.x, a.y, b.x, b.y, thick, color);
    draw_line(b.x, b.y, c.x, c.y, thick, color);
    draw_line(c.x, c.y, a.x, a.y, thick, color);
}

fn draw_head(cx: f32, cy: f32, u: f32, species: &SpeciesDef, pal: &Palette, rng: &mut Rng) {
    let hr = 0.4 * u;
    let hy = cy - 0.52 * u;
    // Ears / horns by temperament, chosen deterministically.
    draw_head_crest(cx, hy, hr, species, pal, rng);
    draw_circle(cx, hy, hr, brighten(pal.base, 0.03));
    draw_circle_lines(cx, hy, hr, 2.0, pal.line);
    if pal.silhouette {
        return;
    }
    // Kawaii face — big eyes with catch-lights, blush, tiny mouth. The precious
    // thing the whole war-body exists to protect (`game_design.md` §10).
    let eye_dx = hr * 0.42;
    let eye_y = hy - hr * 0.05;
    let eye_r = hr * 0.26;
    for s in [-1.0_f32, 1.0] {
        let ex = cx + s * eye_dx;
        draw_circle(ex, eye_y, eye_r, Color::new(0.09, 0.08, 0.11, 1.0));
        draw_circle(ex + eye_r * 0.3, eye_y - eye_r * 0.35, eye_r * 0.42, WHITE);
        draw_circle(
            ex,
            eye_y + eye_r * 1.5,
            eye_r * 0.5,
            Color::new(0.95, 0.55, 0.55, 0.32),
        );
    }
    draw_circle(
        cx,
        hy + hr * 0.5,
        hr * 0.09,
        Color::new(0.35, 0.18, 0.2, 0.9),
    );
}

fn draw_head_crest(cx: f32, hy: f32, hr: f32, species: &SpeciesDef, pal: &Palette, rng: &mut Rng) {
    use crate::data::species::Temperament::*;
    let horn = darken(pal.base, 0.22);
    let inner = mix(pal.base, Color::new(0.95, 0.6, 0.62, 1.0), 0.5);
    match species.temperament {
        Fierce => {
            // Two horns swept back.
            for s in [-1.0_f32, 1.0] {
                let base = vec2(cx + s * hr * 0.6, hy - hr * 0.7);
                draw_triangle(
                    base,
                    base + vec2(s * 0.16 * hr, -1.4 * hr),
                    base + vec2(s * 0.5 * hr, -0.2 * hr),
                    horn,
                );
            }
        }
        Gentle => {
            // Floppy round ears.
            for s in [-1.0_f32, 1.0] {
                draw_circle(
                    cx + s * hr * 0.78,
                    hy - hr * 0.5,
                    hr * 0.34,
                    darken(pal.base, 0.05),
                );
                if !pal.silhouette {
                    draw_circle(cx + s * hr * 0.78, hy - hr * 0.5, hr * 0.18, inner);
                }
            }
        }
        Wary | Placid => {
            // Pricked triangular ears.
            for s in [-1.0_f32, 1.0] {
                let base = vec2(cx + s * hr * 0.55, hy - hr * 0.55);
                draw_triangle(
                    base + vec2(-0.22 * hr, 0.0),
                    base + vec2(0.22 * hr, 0.0),
                    base + vec2(s * 0.15 * hr, -0.95 * hr),
                    darken(pal.base, 0.05),
                );
            }
        }
    }
    let _ = rng;
}

/// Anchor offset (from centre, in units) and outward direction for a region.
fn region_anchor(region: LimbRegion, u: f32) -> (Vec2, Vec2) {
    let (ox, oy, dx, dy) = match region {
        LimbRegion::Head => (0.0, -0.95, 0.0, -1.0),
        LimbRegion::Back => (0.0, -0.3, 0.0, -1.0),
        LimbRegion::ArmLeft => (-0.72, 0.05, -1.0, 0.0),
        LimbRegion::ArmRight => (0.72, 0.05, 1.0, 0.0),
        LimbRegion::Legs => (0.0, 0.72, 0.0, 1.0),
        LimbRegion::Tail => (0.82, 0.42, 1.0, 0.35),
    };
    (vec2(ox * u, oy * u), vec2(dx, dy).normalize())
}

fn draw_grafts(cx: f32, cy: f32, u: f32, grafts: &[GraftVisual]) {
    // Group by region so multiples on one region fan out instead of stacking.
    for region in [
        LimbRegion::Head,
        LimbRegion::Back,
        LimbRegion::ArmLeft,
        LimbRegion::ArmRight,
        LimbRegion::Legs,
        LimbRegion::Tail,
    ] {
        let here: Vec<&GraftVisual> = grafts.iter().filter(|g| g.region == region).collect();
        if here.is_empty() {
            continue;
        }
        let (anchor, dir) = region_anchor(region, u);
        let perp = vec2(-dir.y, dir.x);
        let n = here.len() as f32;
        for (i, g) in here.iter().enumerate() {
            let spread = (i as f32 - (n - 1.0) * 0.5) * 0.34 * u;
            let pos = vec2(cx, cy) + anchor + perp * spread;
            let tint = g
                .element
                .map(element_color)
                .unwrap_or(Color::new(0.7, 0.72, 0.75, 1.0));
            match g.kind {
                GraftKind::Weapon => draw_weapon(pos, dir, u, tint),
                GraftKind::Armor => draw_armor(pos, u, tint),
                GraftKind::Utility => draw_utility(pos, dir, u, tint),
            }
            if g.broken {
                draw_broken_mark(pos, u);
            }
        }
    }
}

fn draw_weapon(pos: Vec2, dir: Vec2, u: f32, tint: Color) {
    let tip = pos + dir * 0.5 * u;
    let metal = Color::new(0.56, 0.58, 0.63, 1.0);
    // Shoulder mount, dark barrel underlay, barrel, glowing muzzle.
    draw_circle(pos.x, pos.y, 0.14 * u, Color::new(0.18, 0.18, 0.2, 1.0));
    draw_line(
        pos.x,
        pos.y,
        tip.x,
        tip.y,
        0.22 * u,
        Color::new(0.2, 0.2, 0.23, 1.0),
    );
    draw_line(pos.x, pos.y, tip.x, tip.y, 0.13 * u, metal);
    draw_circle(tip.x, tip.y, 0.11 * u, brighten(tint, 0.1));
    draw_circle(tip.x, tip.y, 0.05 * u, WHITE);
}

fn draw_armor(pos: Vec2, u: f32, tint: Color) {
    let plate = mix(tint, Color::new(0.5, 0.53, 0.58, 1.0), 0.55);
    draw_poly(pos.x, pos.y, 6, 0.26 * u, 15.0, plate);
    draw_poly_lines(pos.x, pos.y, 6, 0.26 * u, 15.0, 2.0, darken(plate, 0.2));
    draw_circle(pos.x, pos.y, 0.05 * u, brighten(plate, 0.18));
}

fn draw_utility(pos: Vec2, dir: Vec2, u: f32, tint: Color) {
    let base = pos - dir * 0.16 * u;
    draw_line(base.x, base.y, pos.x, pos.y, 0.08 * u, darken(tint, 0.2));
    draw_circle(pos.x, pos.y, 0.2 * u, tint);
    draw_circle_lines(pos.x, pos.y, 0.2 * u, 1.5, darken(tint, 0.22));
    draw_circle(
        pos.x - 0.06 * u,
        pos.y - 0.06 * u,
        0.07 * u,
        Color::new(1.0, 1.0, 1.0, 0.7),
    );
}

fn draw_broken_mark(pos: Vec2, u: f32) {
    let r = 0.16 * u;
    let red = Color::new(0.92, 0.28, 0.24, 0.95);
    draw_line(pos.x - r, pos.y - r, pos.x + r, pos.y + r, 2.5, red);
    draw_line(pos.x - r, pos.y + r, pos.x + r, pos.y - r, 2.5, red);
}
