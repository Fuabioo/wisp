// Phosphor Ghost palette — see docs/adr/0002-cosmic-admin-gui.md.
//
// Brand contract: these accents mirror the lipgloss colors in cmd/root.go
// (`successStyle`/`accentStyle`) so the CLI and GUI share DNA. If you
// change either side, change both — and update ADR 0002.
//
//   lipgloss "99"  (purple)  → accent.wisp   (#9B6EFF)
//   lipgloss "212" (pink)    → accent.brand  (#FF87D7)
//   lipgloss "204" (rose)    → accent.rose   (#FF8FAF)
//
// The styling helpers below are intentional placeholders — the v1.5 polish
// pass wires them into the daemon ribbon, error banner, and selected
// session orb. Marked `dead_code` until then so the warnings don't drown
// out real signal during dev.

#![allow(dead_code)]

use cosmic::iced::Color;
use cosmic::widget::container;

pub const WISP: Color = Color::from_rgb(0x9B as f32 / 255.0, 0x6E as f32 / 255.0, 1.0);
pub const ALIVE: Color = Color::from_rgb(0x7C as f32 / 255.0, 0xE3 as f32 / 255.0, 0xA9 as f32 / 255.0);
pub const ROSE: Color = Color::from_rgb(1.0, 0x8F as f32 / 255.0, 0xAF as f32 / 255.0);
pub const BRAND: Color = Color::from_rgb(1.0, 0x87 as f32 / 255.0, 0xD7 as f32 / 255.0);
pub const DANGER: Color = Color::from_rgb(1.0, 0x6B as f32 / 255.0, 0x7A as f32 / 255.0);

pub fn error_banner_style() -> impl Fn(&cosmic::Theme) -> container::Style + 'static {
    move |_theme| container::Style {
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
