// Phosphor Ghost palette — see docs/adr/0002-cosmic-admin-gui.md.
//
// Brand contract: these accents mirror the lipgloss colors in cmd/root.go
// (`successStyle`/`accentStyle`) so the CLI and GUI share DNA. If you
// change either side, change both — and update ADR 0002.
//
//   lipgloss "99"  (purple)  → accent.wisp   (#9B6EFF)
//   lipgloss "212" (pink)    → accent.brand  (#FF87D7)
//   lipgloss "204" (rose)    → accent.rose   (#FF8FAF)

#![allow(dead_code)]

use cosmic::iced::Color;
use cosmic::widget::container;

pub const WISP: Color = Color::from_rgb(0x9B as f32 / 255.0, 0x6E as f32 / 255.0, 1.0);
pub const ALIVE: Color = Color::from_rgb(0x7C as f32 / 255.0, 0xE3 as f32 / 255.0, 0xA9 as f32 / 255.0);
pub const ROSE: Color = Color::from_rgb(1.0, 0x8F as f32 / 255.0, 0xAF as f32 / 255.0);
pub const BRAND: Color = Color::from_rgb(1.0, 0x87 as f32 / 255.0, 0xD7 as f32 / 255.0);
pub const DANGER: Color = Color::from_rgb(1.0, 0x6B as f32 / 255.0, 0x7A as f32 / 255.0);

/// Container style for the inline error banner: a translucent danger-tinted
/// fill with high-contrast text. Pair with `container(...).style(...)`.
pub fn error_banner_style(_theme: &cosmic::Theme) -> container::Style {
    container::Style {
        background: Some(cosmic::iced::Background::Color(Color {
            r: DANGER.r,
            g: DANGER.g,
            b: DANGER.b,
            a: 0.18,
        })),
        text_color: Some(DANGER),
        ..Default::default()
    }
}

/// Container style for the daemon ribbon at the top of the window — a thin
/// strip with a subtle wisp-tinted background that survives in both light
/// and dark COSMIC themes.
pub fn ribbon_style(_theme: &cosmic::Theme) -> container::Style {
    container::Style {
        background: Some(cosmic::iced::Background::Color(Color {
            r: WISP.r,
            g: WISP.g,
            b: WISP.b,
            a: 0.08,
        })),
        ..Default::default()
    }
}

/// Returns the brand accent colour for "Active" or "Asleep" status text.
pub fn status_color(active: bool) -> Color {
    if active { ALIVE } else { ROSE }
}

/// Catppuccin-Mocha mantle tone — slightly lighter than the window
/// base. Used for surfaces (cards, popups) so they sit visibly above
/// the background while still letting alpha bleed through.
pub const MANTLE: Color = Color::from_rgb(
    0x18 as f32 / 255.0,
    0x18 as f32 / 255.0,
    0x25 as f32 / 255.0,
);

/// Catppuccin-Mocha "Surface 0" — the low-contrast tone we use for
/// the sidebar's right-edge border so the sidebar is delineated from
/// the body without painting an opaque background.
pub const SURFACE0: Color = Color::from_rgb(
    0x31 as f32 / 255.0,
    0x32 as f32 / 255.0,
    0x44 as f32 / 255.0,
);

/// Catppuccin-Mocha base — used as the body tint when the user wants
/// some app colour bleed without blur. Painting it via a wrapping
/// container (rather than as the wgpu clear colour) avoids the
/// transparent-surface re-paint flicker iced exhibits when you set
/// the background_color in `Application::style()`.
pub const BASE: Color = Color::from_rgb(
    0x1e as f32 / 255.0,
    0x1e as f32 / 255.0,
    0x2e as f32 / 255.0,
);

/// Body-wrap container style: paints the Catppuccin base at the
/// caller's alpha (0.0 = fully transparent, wallpaper bleed). This is
/// the *only* place in the chrome that sets an alpha background, so
/// the wgpu surface clear stays at `Color::TRANSPARENT` and the
/// per-frame clear doesn't flicker against the wallpaper.
pub fn body_tint_style(alpha: f32) -> impl Fn(&cosmic::Theme) -> container::Style + 'static {
    let alpha = alpha.clamp(0.0, 1.0);
    move |_theme| container::Style {
        background: if alpha > 0.0 {
            Some(cosmic::iced::Background::Color(Color {
                r: BASE.r,
                g: BASE.g,
                b: BASE.b,
                a: alpha,
            }))
        } else {
            None
        },
        ..Default::default()
    }
}

/// Sidebar container style: always fully transparent so the wallpaper
/// shows through. A single low-contrast right-edge separator is drawn
/// by appending a `vertical_rule` to the sidebar contents — this
/// container itself paints nothing.
pub fn sidebar_style(_theme: &cosmic::Theme) -> container::Style {
    container::Style {
        background: None,
        ..Default::default()
    }
}

/// Style for the 1-pixel container we slot at the right edge of the
/// sidebar so the rail has a visible (but quiet) seam against the
/// wallpaper. A bare container is simpler than wiring the iced
/// `rule::Style` into cosmic's `Theme::Catalog` — at 1 px wide the
/// distinction is invisible.
pub fn sidebar_edge_style(_theme: &cosmic::Theme) -> container::Style {
    container::Style {
        background: Some(cosmic::iced::Background::Color(SURFACE0)),
        ..Default::default()
    }
}

/// Surface style for cards / popups. Uses Catppuccin-Mocha mantle tinted
/// at `alpha`; pair with `Settings::effective_alpha()` so the user's
/// opacity slider cascades through the chrome instead of being masked
/// by opaque containers.
pub fn panel_style(alpha: f32) -> impl Fn(&cosmic::Theme) -> container::Style + 'static {
    let alpha = alpha.clamp(0.0, 1.0);
    move |_theme| container::Style {
        background: Some(cosmic::iced::Background::Color(Color {
            r: MANTLE.r,
            g: MANTLE.g,
            b: MANTLE.b,
            a: alpha,
        })),
        text_color: Some(Color::from_rgb(
            0xcd as f32 / 255.0,
            0xd6 as f32 / 255.0,
            0xf4 as f32 / 255.0,
        )),
        border: cosmic::iced::Border {
            radius: 8.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}
