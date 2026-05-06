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
pub fn error_banner_style(theme: &cosmic::Theme) -> container::Style {
    container::Style {
        background: Some(cosmic::iced::Background::Color(Color {
            r: DANGER.r,
            g: DANGER.g,
            b: DANGER.b,
            a: 0.18,
        })),
        text_color: Some(theme.cosmic().on_bg_color().into()),
        ..Default::default()
    }
}

/// Container style for the daemon ribbon at the top of the window — a thin
/// strip with a subtle wisp-tinted background that survives in both light
/// and dark COSMIC themes.
pub fn ribbon_style(theme: &cosmic::Theme) -> container::Style {
    let alpha = if theme.cosmic().is_dark { 0.08 } else { 0.14 };
    container::Style {
        background: Some(cosmic::iced::Background::Color(Color {
            r: WISP.r,
            g: WISP.g,
            b: WISP.b,
            a: alpha,
        })),
        ..Default::default()
    }
}

/// Returns the brand accent colour for "Active" or "Asleep" status text.
pub fn status_color(active: bool) -> Color {
    if active { ALIVE } else { ROSE }
}

/// Body-wrap container style: paints the system background colour at the
/// caller's alpha (0.0 = fully transparent, wallpaper bleed). This is
/// the *only* place in the chrome that sets an alpha background, so
/// the wgpu surface clear stays at `Color::TRANSPARENT` and the
/// per-frame clear doesn't flicker against the wallpaper.
pub fn body_tint_style(alpha: f32) -> impl Fn(&cosmic::Theme) -> container::Style + 'static {
    let alpha = alpha.clamp(0.0, 1.0);
    move |theme| container::Style {
        background: if alpha > 0.0 {
            let bg: cosmic::iced::Color = theme.cosmic().bg_color().into();
            Some(cosmic::iced::Background::Color(Color {
                r: bg.r,
                g: bg.g,
                b: bg.b,
                a: alpha,
            }))
        } else {
            None
        },
        ..Default::default()
    }
}

/// Style for the 1-pixel container we slot at the right edge of the
/// sidebar so the rail has a visible (but quiet) seam against the
/// wallpaper. A bare container is simpler than wiring the iced
/// `rule::Style` into cosmic's `Theme::Catalog` — at 1 px wide the
/// distinction is invisible.
pub fn sidebar_edge_style(theme: &cosmic::Theme) -> container::Style {
    container::Style {
        background: Some(cosmic::iced::Background::Color(
            theme.cosmic().bg_divider().into(),
        )),
        ..Default::default()
    }
}

/// Surface style for cards / popups. Uses the system theme's primary
/// container colours tinted at `alpha`; pair with
/// `Settings::effective_alpha()` so the user's opacity slider cascades
/// through the chrome instead of being masked by opaque containers.
pub fn panel_style(alpha: f32) -> impl Fn(&cosmic::Theme) -> container::Style + 'static {
    let alpha = alpha.clamp(0.0, 1.0);
    move |theme| {
        let bg: cosmic::iced::Color = theme.cosmic().primary_container_color().into();
        let on_bg: cosmic::iced::Color = theme.cosmic().on_primary_container_color().into();
        container::Style {
            background: Some(cosmic::iced::Background::Color(Color {
                r: bg.r,
                g: bg.g,
                b: bg.b,
                a: alpha,
            })),
            text_color: Some(on_bg),
            border: cosmic::iced::Border {
                radius: 8.0.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    }
}
