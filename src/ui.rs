//! View layer: screens read state and return intents; game logic applies them.

pub mod battle;
pub mod outfit;
pub mod overworld;
pub mod settlement;

use crate::data::species::Element;
use crate::data::GameData;
use crate::state::GameSession;
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::{draw_ui_text_ex, RectExt, VirtualUi};

pub const LOGICAL_WIDTH: f32 = 1280.0;
pub const LOGICAL_HEIGHT: f32 = 720.0;

/// Intents the menu returns to the game loop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiAction {
    NewGame,
    EnterWorld,
    StartDevBattle,
    OpenOutfit,
    Save,
    Load,
    TogglePace,
}

pub fn element_color(element: Element) -> Color {
    match element {
        Element::BioElectric => Color::new(0.95, 0.85, 0.30, 1.0),
        Element::Plant => Color::new(0.45, 0.80, 0.40, 1.0),
        Element::Rock => Color::new(0.62, 0.55, 0.45, 1.0),
        Element::Fire => Color::new(0.92, 0.45, 0.25, 1.0),
        Element::Water => Color::new(0.35, 0.60, 0.90, 1.0),
        Element::Poison => Color::new(0.70, 0.45, 0.85, 1.0),
    }
}

pub struct MenuContext<'a> {
    pub data: &'a GameData,
    pub session: &'a GameSession,
    pub save_exists: bool,
    pub ui: &'a VirtualUi,
}

pub fn draw_main_menu(ctx: &MenuContext<'_>) -> Vec<UiAction> {
    let mut actions = Vec::new();
    let mouse = ctx.ui.mouse_position();

    // Title block.
    draw_ui_text_ex(
        "IRON FAUNA",
        80.0,
        160.0,
        TextStyle::new(64.0, Color::new(0.85, 0.88, 0.92, 1.0)).params(),
    );
    draw_ui_text_ex(
        "hold the line. mind what it costs.",
        84.0,
        200.0,
        TextStyle::new(20.0, dark::TEXT_DIM).params(),
    );

    let stats = format!(
        "{} species catalogued   ·   {} graftware patterns   ·   party {}/{} slots",
        ctx.data.species.len(),
        ctx.data.graftware.len(),
        ctx.session.profile.roster.slots_used(ctx.data),
        ctx.data.balance.party_slot_budget,
    );
    draw_ui_text_ex(
        &stats,
        84.0,
        240.0,
        TextStyle::new(16.0, dark::TEXT_DIM).params(),
    );

    let buttons: [(&str, UiAction, bool); 7] = [
        ("Enter the World", UiAction::EnterWorld, true),
        ("New Game", UiAction::NewGame, true),
        ("Dev Battle", UiAction::StartDevBattle, true),
        ("Party & Grafting", UiAction::OpenOutfit, true),
        ("Save", UiAction::Save, true),
        ("Load", UiAction::Load, ctx.save_exists),
        (
            match ctx.session.pace {
                crate::state::PaceSetting::Wait => "Pace: Wait",
                crate::state::PaceSetting::Active => "Pace: Active",
            },
            UiAction::TogglePace,
            true,
        ),
    ];

    for (i, (label, action, enabled)) in buttons.iter().enumerate() {
        let rect = Rect::new(84.0, 300.0 + i as f32 * 58.0, 280.0, 44.0);
        if menu_button(rect, label, *enabled, mouse) {
            actions.push(action.clone());
        }
    }

    // Party summary panel.
    let panel = Rect::new(520.0, 290.0, 660.0, 330.0);
    let style = SurfaceStyle::new(Color::new(0.08, 0.085, 0.105, 0.97))
        .with_border(1.0, Color::new(0.38, 0.45, 0.58, 0.65))
        .with_header(38.0, Color::new(0.105, 0.12, 0.15, 1.0))
        .with_header_divider(1.0, Color::new(0.38, 0.45, 0.58, 0.4));
    draw_surface_with_title(
        panel,
        Some("Traveling Party"),
        &style,
        TextStyle::new(17.0, dark::TEXT),
    );
    let content = panel.inset(18.0);
    let mut y = content.y + 44.0;
    for creature in ctx.session.profile.roster.party_members() {
        let species = creature.species(ctx.data);
        draw_circle(
            content.x + 12.0,
            y - 6.0,
            9.0,
            element_color(species.element),
        );
        draw_ui_text_ex(
            &format!(
                "{}  ·  {}  {}  ·  Power {}  Speed {}  ·  bond {:.1}",
                creature.display_name(ctx.data),
                species.size.display_name(),
                species.archetype.display_name(),
                species.power,
                species.speed,
                creature.bond,
            ),
            content.x + 32.0,
            y,
            TextStyle::new(16.0, dark::TEXT).params(),
        );
        y += 30.0;
    }
    draw_ui_text_ex(
        &format!(
            "scrip: {}   ·   parts owned: {}   ·   battles: {}",
            ctx.session.profile.inventory.scrip,
            ctx.session.profile.inventory.items.len(),
            ctx.session.battles_fought,
        ),
        content.x,
        content.bottom() - 14.0,
        TextStyle::new(15.0, dark::TEXT_DIM).params(),
    );

    actions
}

pub fn menu_button(rect: Rect, text: &str, enabled: bool, mouse: Vec2) -> bool {
    let hovered = enabled && rect.contains_point(mouse);
    let pressed = hovered && is_mouse_button_down(MouseButton::Left);
    let fill = if !enabled {
        Color::new(0.10, 0.11, 0.13, 1.0)
    } else if pressed {
        Color::new(0.20, 0.26, 0.34, 1.0)
    } else if hovered {
        Color::new(0.16, 0.20, 0.27, 1.0)
    } else {
        Color::new(0.12, 0.14, 0.18, 1.0)
    };
    draw_surface(
        rect,
        &SurfaceStyle::new(fill).with_border(1.0, Color::new(0.45, 0.52, 0.65, 0.6)),
    );
    draw_text_centered_in_box_ex(
        text,
        rect.x + 8.0,
        rect.y + if pressed { 2.0 } else { 0.0 },
        rect.w - 16.0,
        rect.h,
        TextStyle::new(
            18.0,
            if enabled {
                dark::TEXT_BRIGHT
            } else {
                dark::TEXT_DIM
            },
        ),
    );
    hovered && is_mouse_button_released(MouseButton::Left)
}
