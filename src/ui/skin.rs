//! Sprite-backed UI skin from the Flat UI Essential pack: nine-slice buttons,
//! panels, and a title banner.
//!
//! The loaded textures live in a thread-local, write-once cache so any draw
//! function can reach them without threading the `AssetManager` through every
//! screen signature. This is a read-only asset cache populated at startup — not
//! mutable game state — and macroquad is single-threaded, so a thread-local is
//! the right shape. When textures are absent (WASM without assets, load
//! failure) the helpers report unavailable and callers fall back to the
//! hand-drawn look.

use macroquad::prelude::*;
use macroquad_toolkit::assets::AssetManager;
use macroquad_toolkit::prelude::*;
use std::cell::RefCell;

#[derive(Clone)]
struct Skin {
    btn: Texture2D,
    btn_hover: Texture2D,
    btn_press: Texture2D,
    btn_disabled: Texture2D,
    frame: Texture2D,
    banner: Texture2D,
}

thread_local! {
    static SKIN: RefCell<Option<Skin>> = const { RefCell::new(None) };
}

/// Populate the skin cache from loaded textures. A no-op (leaving the skin
/// unavailable) if any piece is missing.
pub fn init(assets: &AssetManager) {
    let get = |k: &str| assets.get_texture(k).cloned();
    if let (
        Some(btn),
        Some(btn_hover),
        Some(btn_press),
        Some(btn_disabled),
        Some(frame),
        Some(banner),
    ) = (
        get("ui_btn"),
        get("ui_btn_hover"),
        get("ui_btn_press"),
        get("ui_btn_disabled"),
        get("ui_frame"),
        get("ui_banner"),
    ) {
        SKIN.with(|s| {
            *s.borrow_mut() = Some(Skin {
                btn,
                btn_hover,
                btn_press,
                btn_disabled,
                frame,
                banner,
            })
        });
    }
}

/// Whether the sprite skin is loaded and usable.
pub fn available() -> bool {
    SKIN.with(|s| s.borrow().is_some())
}

/// Nine-slice blit: corners stay fixed-size, edges and centre stretch to fill
/// `rect`. `m` is the source (and destination) corner size in pixels.
fn nine_slice(tex: &Texture2D, rect: Rect, m: f32) {
    let (tw, th) = (tex.width(), tex.height());
    // Source column/row boundaries.
    let sx = [0.0, m, tw - m, tw];
    let sy = [0.0, m, th - m, th];
    // Destination boundaries — corners fixed, middle takes the slack.
    let dx = [rect.x, rect.x + m, rect.right() - m, rect.right()];
    let dy = [rect.y, rect.y + m, rect.bottom() - m, rect.bottom()];
    for r in 0..3 {
        for c in 0..3 {
            let src = Rect::new(sx[c], sy[r], sx[c + 1] - sx[c], sy[r + 1] - sy[r]);
            let dw = dx[c + 1] - dx[c];
            let dh = dy[r + 1] - dy[r];
            if dw <= 0.0 || dh <= 0.0 {
                continue;
            }
            draw_texture_ex(
                tex,
                dx[c],
                dy[r],
                WHITE,
                DrawTextureParams {
                    source: Some(src),
                    dest_size: Some(vec2(dw, dh)),
                    ..Default::default()
                },
            );
        }
    }
}

/// Draw a skinned panel background. Caller still draws its own contents.
pub fn panel(rect: Rect) {
    SKIN.with(|s| {
        if let Some(skin) = s.borrow().as_ref() {
            nine_slice(&skin.frame, rect, 12.0);
        }
    });
}

/// Draw a skinned title banner with centred dark text.
pub fn banner(rect: Rect, text: &str, size: f32) {
    SKIN.with(|s| {
        if let Some(skin) = s.borrow().as_ref() {
            nine_slice(&skin.banner, rect, 8.0);
            draw_text_centered_in_box_ex(
                text,
                rect.x,
                rect.y - 1.0,
                rect.w,
                rect.h,
                TextStyle::new(size, Color::new(0.16, 0.12, 0.08, 1.0)),
            );
        }
    });
}

/// A skinned push-button. Returns true on release-over. Mirrors the fallback
/// `menu_button` contract so it drops in transparently.
pub fn button(rect: Rect, text: &str, enabled: bool, mouse: Vec2) -> bool {
    let hovered = enabled && rect.contains(mouse);
    let pressed = hovered && is_mouse_button_down(MouseButton::Left);
    SKIN.with(|s| {
        if let Some(skin) = s.borrow().as_ref() {
            let tex = if !enabled {
                &skin.btn_disabled
            } else if pressed {
                &skin.btn_press
            } else if hovered {
                &skin.btn_hover
            } else {
                &skin.btn
            };
            nine_slice(tex, rect, 8.0);
            // The button face is cream — dark text reads on it. The sprite's
            // orange lip grows downward on press, so nudge the label with it.
            let dip = if pressed { 2.0 } else { 0.0 };
            let color = if enabled {
                Color::new(0.14, 0.11, 0.08, 1.0)
            } else {
                Color::new(0.45, 0.42, 0.38, 1.0)
            };
            draw_text_centered_in_box_ex(
                text,
                rect.x,
                rect.y - 3.0 + dip,
                rect.w,
                rect.h,
                TextStyle::new(18.0, color),
            );
        }
    });
    hovered && is_mouse_button_released(MouseButton::Left)
}
