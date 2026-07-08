//! Per-tab rendering for the Codex. Each function paints one tab into the body
//! rect and pushes any `CodexAction` intents the player triggered.

use super::{CodexAction, CodexScreen};
use crate::data::GameData;
use crate::model::creature::{CreatureInstance, CreatureOrigin};
use crate::model::journal;
use crate::model::rider::RiderUpgrade;
use crate::state::GameSession;
use crate::ui::{creature_art, element_color, menu_button};
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::{draw_ui_text_ex, RectExt};

fn text(s: &str, x: f32, y: f32, size: f32, c: Color) {
    draw_ui_text_ex(s, x, y, TextStyle::new(size, c).params());
}

fn origin_label(origin: &CreatureOrigin) -> &'static str {
    match origin {
        CreatureOrigin::Wild => "caught in the wild",
        CreatureOrigin::Freed { .. } => "freed from a war-body",
        CreatureOrigin::Grown { .. } => "grown in a bound vat",
        CreatureOrigin::Starter => "your first core",
    }
}

// ---------------------------------------------------------------- Status

pub(super) fn status(data: &GameData, session: &GameSession, c: Rect) {
    let p = &session.profile;
    text(
        &format!("Rider {}", p.rider.name),
        c.x,
        c.y + 8.0,
        24.0,
        dark::TEXT_BRIGHT,
    );

    // Left column: rider progression.
    let mut y = c.y + 52.0;
    text(
        "Gestarium marks",
        c.x,
        y,
        16.0,
        Color::new(0.75, 0.85, 0.55, 1.0),
    );
    y += 24.0;
    for up in RiderUpgrade::ALL {
        let earned = p.rider.has(up);
        text(
            &format!(
                "{}  {} — {}",
                if earned { "woven" } else { "—" },
                up.display_name(),
                up.description()
            ),
            c.x,
            y,
            13.0,
            if earned { dark::TEXT } else { dark::TEXT_DIM },
        );
        y += 20.0;
    }
    y += 12.0;
    text(
        "Ring ranks",
        c.x,
        y,
        16.0,
        Color::new(0.75, 0.85, 0.55, 1.0),
    );
    y += 22.0;
    if p.rider.duel_ranks.is_empty() {
        text("no rings entered yet", c.x, y, 13.0, dark::TEXT_DIM);
    } else {
        for (town, rank) in &p.rider.duel_ranks {
            let name = data
                .settlements
                .get(town)
                .map(|s| s.name.as_str())
                .unwrap_or(town.as_str());
            text(
                &format!("{} — rank {}", name, rank),
                c.x,
                y,
                13.0,
                dark::TEXT,
            );
            y += 18.0;
        }
    }

    // Right column: tallies.
    let rx = c.x + c.w * 0.56;
    let caught = data
        .species
        .iter()
        .filter(|(id, _)| session.profile.roster.owns_species(id))
        .count();
    let verdicts = data
        .factories
        .iter()
        .filter(|(id, _)| session.world_state.factory(id).verdict.is_some())
        .count();
    let relapsed = data
        .factories
        .iter()
        .filter(|(id, _)| session.world_state.factory(id).relapsed)
        .count();
    let rows = [
        (
            "Cores catalogued",
            format!("{} / {}", caught, data.species.len()),
        ),
        ("Battles fought", session.battles_fought.to_string()),
        ("Steps walked", session.steps.to_string()),
        ("Scrip", p.inventory.scrip.to_string()),
        ("Graftware owned", p.inventory.items.len().to_string()),
        (
            "Verdicts passed",
            format!("{} / {}", verdicts, data.factories.len()),
        ),
        ("Regions relapsed", relapsed.to_string()),
    ];
    let mut ry = c.y + 52.0;
    text("Standing", rx, ry, 16.0, Color::new(0.75, 0.85, 0.55, 1.0));
    ry += 26.0;
    for (label, val) in rows {
        text(label, rx, ry, 15.0, dark::TEXT_DIM);
        text(&val, rx + 240.0, ry, 15.0, dark::TEXT_BRIGHT);
        ry += 26.0;
    }
}

// ------------------------------------------------------------- Corelings

pub(super) fn corelings(
    screen: &CodexScreen,
    data: &GameData,
    session: &GameSession,
    c: Rect,
    mouse: Vec2,
    actions: &mut Vec<CodexAction>,
) {
    // Left: selectable roster list.
    let list_w = 300.0;
    let mut y = c.y;
    let mut row = |cr: &CreatureInstance, in_party: bool, y: &mut f32| {
        let rect = Rect::new(c.x, *y, list_w, 40.0);
        let selected = screen.selected == Some(cr.id);
        let hovered = rect.contains_point(mouse);
        draw_surface(
            rect,
            &SurfaceStyle::new(if selected {
                Color::new(0.18, 0.24, 0.32, 1.0)
            } else if hovered {
                Color::new(0.14, 0.17, 0.22, 1.0)
            } else {
                Color::new(0.10, 0.115, 0.15, 1.0)
            })
            .with_left_accent(3.0, element_color(cr.species(data).element)),
        );
        text(
            cr.display_name(data),
            rect.x + 12.0,
            rect.y + 18.0,
            15.0,
            dark::TEXT,
        );
        text(
            if in_party { "party" } else { "storage" },
            rect.x + 12.0,
            rect.y + 33.0,
            11.0,
            dark::TEXT_DIM,
        );
        if hovered && is_mouse_button_released(MouseButton::Left) {
            actions.push(CodexAction::Select(cr.id));
        }
        *y += 44.0;
    };
    for cr in session.profile.roster.party_members() {
        row(cr, true, &mut y);
    }
    for cr in session.profile.roster.stored() {
        if y > c.bottom() - 40.0 {
            break;
        }
        row(cr, false, &mut y);
    }

    // Right: selected coreling detail.
    let panel = Rect::new(c.x + list_w + 20.0, c.y, c.w - list_w - 20.0, c.h);
    let Some(cr) = screen
        .selected
        .and_then(|id| session.profile.roster.creature(id))
    else {
        text(
            "Select a coreling to inspect its war-body.",
            panel.x + 10.0,
            panel.y + 40.0,
            15.0,
            dark::TEXT_DIM,
        );
        return;
    };
    let sp = cr.species(data);
    let d = sp.derived(&data.balance);

    // Portrait with the live loadout.
    let grafts = creature_art::grafts_for_creature(data, session, cr);
    let frame = Rect::new(panel.x, panel.y, 190.0, 190.0);
    draw_surface(
        frame,
        &SurfaceStyle::new(Color::new(0.055, 0.065, 0.085, 1.0))
            .with_border(1.0, Color::new(0.3, 0.34, 0.42, 0.6)),
    );
    creature_art::draw_war_body(frame.x + 95.0, frame.y + 100.0, 54.0, sp, &grafts);

    let tx = panel.x + 210.0;
    text(
        cr.display_name(data),
        tx,
        panel.y + 24.0,
        22.0,
        element_color(sp.element),
    );
    text(
        &format!(
            "{} {} · {} · {}",
            sp.size.display_name(),
            sp.archetype.display_name(),
            sp.element.display_name(),
            sp.temperament.display_name(),
        ),
        tx,
        panel.y + 48.0,
        14.0,
        dark::TEXT_DIM,
    );
    let stats = [
        format!("Power {}   Speed {}", sp.power, sp.speed),
        format!(
            "Vigor {:.0}   Strain {:.0}",
            d.vigor_max, d.strain_threshold
        ),
        format!("Core {:.0}   Limb {:.0}", d.core_hp, d.limb_hp),
        format!("Mounts {}   Bond {:.1}", sp.mount_count(), cr.bond),
        format!("Origin: {}", origin_label(&cr.origin)),
    ];
    let mut sy = panel.y + 78.0;
    for line in stats {
        text(&line, tx, sy, 15.0, dark::TEXT);
        sy += 24.0;
    }

    // Loadout.
    sy += 8.0;
    text("Grafted", tx, sy, 15.0, Color::new(0.75, 0.85, 0.55, 1.0));
    sy += 22.0;
    if cr.loadout.is_empty() {
        text(
            "bare chassis — nothing mounted",
            tx,
            sy,
            13.0,
            dark::TEXT_DIM,
        );
    } else {
        for m in &cr.loadout {
            let name = session
                .profile
                .inventory
                .item(m.item_id)
                .and_then(|i| data.graftware.get(&i.def_id))
                .map(|g| g.name.clone())
                .unwrap_or_else(|| "?".to_owned());
            let limb = sp
                .limb(&m.limb_id)
                .map(|l| l.name.as_str())
                .unwrap_or("limb");
            text(&format!("· {} on {}", name, limb), tx, sy, 13.0, dark::TEXT);
            sy += 18.0;
        }
    }
}

// ----------------------------------------------------------------- Party

pub(super) fn party(
    data: &GameData,
    session: &GameSession,
    c: Rect,
    mouse: Vec2,
    actions: &mut Vec<CodexAction>,
) {
    let roster = &session.profile.roster;
    text(
        &format!(
            "Party order — {}/{} field slots. Top of the list leads.",
            roster.slots_used(data),
            data.balance.party_slot_budget
        ),
        c.x,
        c.y + 6.0,
        16.0,
        dark::TEXT,
    );
    let mut y = c.y + 34.0;
    let party: Vec<u64> = roster.party.clone();
    for (i, id) in party.iter().enumerate() {
        let Some(cr) = roster.creature(*id) else {
            continue;
        };
        let row = Rect::new(c.x, y, 620.0, 46.0);
        draw_surface(
            row,
            &SurfaceStyle::new(Color::new(0.11, 0.13, 0.16, 1.0))
                .with_left_accent(3.0, element_color(cr.species(data).element)),
        );
        text(
            &format!("{}. {}", i + 1, cr.display_name(data)),
            row.x + 12.0,
            row.y + 20.0,
            15.0,
            dark::TEXT,
        );
        text(
            &format!(
                "{} · cost {}",
                cr.species(data).size.display_name(),
                cr.species(data).size.slot_cost()
            ),
            row.x + 12.0,
            row.y + 37.0,
            12.0,
            dark::TEXT_DIM,
        );
        if menu_button(
            Rect::new(row.right() - 190.0, row.y + 8.0, 54.0, 30.0),
            "Up",
            i > 0,
            mouse,
        ) {
            actions.push(CodexAction::MoveUp(*id));
        }
        if menu_button(
            Rect::new(row.right() - 130.0, row.y + 8.0, 54.0, 30.0),
            "Down",
            i + 1 < party.len(),
            mouse,
        ) {
            actions.push(CodexAction::MoveDown(*id));
        }
        if menu_button(
            Rect::new(row.right() - 70.0, row.y + 8.0, 62.0, 30.0),
            "Store",
            party.len() > 1,
            mouse,
        ) {
            actions.push(CodexAction::ToStorage(*id));
        }
        y += 52.0;
    }

    // Storage — bring reserves into the line.
    y += 10.0;
    text("Reserves", c.x, y, 15.0, Color::new(0.75, 0.85, 0.55, 1.0));
    y += 22.0;
    let slots_free = roster.slots_free(data);
    let stored: Vec<u64> = roster.stored().map(|cr| cr.id).collect();
    if stored.is_empty() {
        text("(none in storage)", c.x, y, 13.0, dark::TEXT_DIM);
    }
    for id in stored {
        if y > c.bottom() - 40.0 {
            break;
        }
        let Some(cr) = roster.creature(id) else {
            continue;
        };
        let row = Rect::new(c.x, y, 620.0, 40.0);
        draw_surface(row, &SurfaceStyle::new(Color::new(0.10, 0.115, 0.15, 1.0)));
        text(
            cr.display_name(data),
            row.x + 12.0,
            row.y + 24.0,
            15.0,
            dark::TEXT,
        );
        let fits = cr.species(data).size.slot_cost() <= slots_free;
        if menu_button(
            Rect::new(row.right() - 70.0, row.y + 6.0, 62.0, 28.0),
            "Take",
            fits,
            mouse,
        ) {
            actions.push(CodexAction::ToParty(id));
        }
        y += 46.0;
    }
}

// ----------------------------------------------------------------- Items

pub(super) fn items(
    data: &GameData,
    session: &GameSession,
    c: Rect,
    mouse: Vec2,
    actions: &mut Vec<CodexAction>,
) {
    let inv = &session.profile.inventory;
    text(
        "Consumables — potions and ammo are used in combat; field kits on the road.",
        c.x,
        c.y + 6.0,
        15.0,
        dark::TEXT_DIM,
    );
    let mut y = c.y + 36.0;
    if inv.consumables.is_empty() {
        text(
            "The bag is empty. Buy supplies at a settlement post.",
            c.x,
            y,
            14.0,
            dark::TEXT_DIM,
        );
    }
    for (def_id, count) in &inv.consumables {
        let Some(def) = data.items.get(def_id) else {
            continue;
        };
        if y > c.bottom() - 60.0 {
            break;
        }
        text(
            &format!("{}  ×{}   [{}]", def.name, count, def.category()),
            c.x,
            y,
            16.0,
            dark::TEXT_BRIGHT,
        );
        text(&def.description, c.x + 16.0, y + 18.0, 13.0, dark::TEXT_DIM);
        y += 44.0;
    }

    // Field repair: fix a damaged graft with a kit, anywhere.
    let damaged = inv.items.iter().filter(|i| !i.is_usable()).count();
    let kits = inv.consumable_count("repair_kit");
    let btn = Rect::new(c.x, c.bottom() - 44.0, 360.0, 36.0);
    if kits > 0 && damaged > 0 {
        if menu_button(
            btn,
            &format!("Use Repair Kit — mend a damaged graft ({} hurt)", damaged),
            true,
            mouse,
        ) {
            actions.push(CodexAction::FieldRepair);
        }
    } else {
        let note = if damaged == 0 {
            "No damaged graftware.".to_owned()
        } else {
            "No repair kits in the bag.".to_owned()
        };
        text(&note, c.x, c.bottom() - 20.0, 13.0, dark::TEXT_DIM);
    }
}

// ------------------------------------------------------------- Equipment

pub(super) fn equipment(
    data: &GameData,
    session: &GameSession,
    c: Rect,
    at_settlement: bool,
    mouse: Vec2,
    actions: &mut Vec<CodexAction>,
) {
    if at_settlement {
        text(
            "You're at a settlement — the full grafting bench is open.",
            c.x,
            c.y + 8.0,
            16.0,
            dark::TEXT,
        );
        if menu_button(
            Rect::new(c.x, c.y + 40.0, 340.0, 44.0),
            "Open Grafting Bench",
            true,
            mouse,
        ) {
            actions.push(CodexAction::OpenBench);
        }
        text(
            "Mount and swap graftware, repair parts, and rearrange the party.",
            c.x,
            c.y + 104.0,
            14.0,
            dark::TEXT_DIM,
        );
        return;
    }

    // On the road: no full grafting, just a read-only loadout and field repairs.
    text(
        "On the road. Full grafting only at a settlement bench — field repairs are in the Items tab.",
        c.x,
        c.y + 8.0,
        15.0,
        dark::TEXT_DIM,
    );
    let mut y = c.y + 40.0;
    for cr in session.profile.roster.party_members() {
        if y > c.bottom() - 24.0 {
            break;
        }
        let sp = cr.species(data);
        text(
            cr.display_name(data),
            c.x,
            y,
            16.0,
            element_color(sp.element),
        );
        y += 20.0;
        if cr.loadout.is_empty() {
            text("bare chassis", c.x + 16.0, y, 13.0, dark::TEXT_DIM);
            y += 20.0;
        }
        for m in &cr.loadout {
            let name = session
                .profile
                .inventory
                .item(m.item_id)
                .and_then(|i| data.graftware.get(&i.def_id))
                .map(|g| g.name.clone())
                .unwrap_or_else(|| "?".to_owned());
            let limb = sp
                .limb(&m.limb_id)
                .map(|l| l.name.as_str())
                .unwrap_or("limb");
            text(
                &format!("· {} on {}", name, limb),
                c.x + 16.0,
                y,
                13.0,
                dark::TEXT,
            );
            y += 18.0;
        }
        y += 8.0;
    }
}

// ---------------------------------------------------------------- Quests

pub(super) fn quests(data: &GameData, session: &GameSession, c: Rect) {
    let q = &session.quests;
    let mut y = c.y + 6.0;
    let section = |title: &str, y: &mut f32| {
        text(title, c.x, *y, 16.0, Color::new(0.75, 0.85, 0.55, 1.0));
        *y += 24.0;
    };

    section("Active", &mut y);
    if q.active.is_empty() && q.ready.is_empty() {
        text(
            "No bounties in hand. Ask around the settlements.",
            c.x,
            y,
            14.0,
            dark::TEXT_DIM,
        );
        y += 24.0;
    }
    for (id, progress) in &q.active {
        let Some(def) = data.quests.get(id) else {
            continue;
        };
        text(&def.name, c.x, y, 15.0, dark::TEXT_BRIGHT);
        text(
            &format!("{}  ({}/{})", def.tracker, progress, def.objective.count),
            c.x + 16.0,
            y + 18.0,
            13.0,
            dark::TEXT_DIM,
        );
        y += 42.0;
    }
    for id in &q.ready {
        let Some(def) = data.quests.get(id) else {
            continue;
        };
        text(&def.name, c.x, y, 15.0, Color::new(0.95, 0.82, 0.45, 1.0));
        text(
            &format!("Objective met — return to {}", def.giver),
            c.x + 16.0,
            y + 18.0,
            13.0,
            Color::new(0.9, 0.78, 0.5, 1.0),
        );
        y += 42.0;
    }

    y += 10.0;
    section("Finished", &mut y);
    if q.done.is_empty() {
        text("nothing seen through yet", c.x, y, 13.0, dark::TEXT_DIM);
    }
    for id in &q.done {
        if y > c.bottom() - 24.0 {
            break;
        }
        let name = data.quests.get(id).map(|d| d.name.as_str()).unwrap_or(id);
        text(&format!("· {}", name), c.x, y, 14.0, dark::TEXT_DIM);
        y += 22.0;
    }
}

// --------------------------------------------------------------- Journal

// ---------------------------------------------------------------- System

/// Save / load / quit — the in-game system menu. The title screen owns fresh
/// starts and settings; this is where an active run is written or abandoned.
pub(super) fn system(session: &GameSession, c: Rect, mouse: Vec2, actions: &mut Vec<CodexAction>) {
    text(
        &format!(
            "This run  ·  {} steps travelled  ·  {} battles fought",
            session.steps, session.battles_fought
        ),
        c.x,
        c.y + 8.0,
        16.0,
        dark::TEXT_DIM,
    );

    let bw = 260.0;
    let bh = 46.0;
    let mut y = c.y + 44.0;
    for (label, action) in [
        ("Save Game", CodexAction::Save),
        ("Load Game", CodexAction::Load),
        ("Exit Game", CodexAction::ExitGame),
    ] {
        if menu_button(Rect::new(c.x, y, bw, bh), label, true, mouse) {
            actions.push(action);
        }
        y += bh + 12.0;
    }

    text(
        "Exit quits to desktop without saving — save first if you want to keep this run.",
        c.x,
        y + 6.0,
        14.0,
        dark::TEXT_DIM,
    );
}

pub(super) fn journal(data: &GameData, session: &GameSession, c: Rect) {
    // Current step, highlighted.
    let gold = Color::new(0.95, 0.82, 0.45, 1.0);
    draw_rectangle(c.x, c.y, c.w, 34.0, Color::new(0.0, 0.0, 0.0, 0.4));
    draw_rectangle(c.x, c.y, 4.0, 34.0, gold);
    text("Right now:", c.x + 14.0, c.y + 22.0, 15.0, gold);
    text(
        &journal::current_step(data, session),
        c.x + 130.0,
        c.y + 22.0,
        15.0,
        dark::TEXT_BRIGHT,
    );

    // Chronicle, newest first.
    let mut y = c.y + 58.0;
    text("Chronicle", c.x, y, 16.0, Color::new(0.75, 0.85, 0.55, 1.0));
    y += 24.0;
    for entry in session.journal.iter().rev() {
        if y > c.bottom() - 20.0 {
            break;
        }
        text(
            &format!("step {}", entry.step),
            c.x,
            y,
            12.0,
            dark::TEXT_DIM,
        );
        text(&entry.text, c.x + 80.0, y, 14.0, dark::TEXT);
        y += 22.0;
    }
}
