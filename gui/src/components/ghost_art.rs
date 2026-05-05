// Pixel ghost — the brand leitmotif. Loads `assets/logo-wisp.svg` directly
// (see ADR 0002's brand-DNA contract).
//
// The source SVG carries seven `<animate>` elements but iced's SVG renderer
// goes through resvg/usvg which doesn't run SMIL. To recreate the look, we
// strip the `<animate>` tags at startup, then pre-bake N static frames with
// the gradient stop colours, stop offset, and gradient endpoints
// substituted to their values at evenly-spaced points along an 11-second
// master cycle (the longest individual animation period). The SMIL opacity
// animation is intentionally dropped — the gradient flow alone reads as
// "wisp shimmer" without the breathing distraction.

use std::sync::OnceLock;

use cosmic::iced::Length;
use cosmic::widget::svg::{self, Svg};
use cosmic::Element;

const ORIGINAL: &str = include_str!("../../../assets/logo-wisp.svg");

/// Number of pre-baked frames cycled through at render time. 64 frames
/// over the 11s master cycle works out to ~6 fps of visible animation —
/// not browser-smooth but enough to read as a wisp shimmer.
const FRAMES: usize = 64;

/// Master loop period — chosen as the longest SMIL period (the teal
/// stop-colour at 11s) so all sub-animations complete at least one full
/// cycle within it.
const MASTER_CYCLE_SECS: f32 = 11.0;

const X1_KEYS: &[(f32, f32)] = &[(0.0, 132.5), (0.33, 115.0), (0.66, 150.0), (1.0, 132.5)];
const X2_KEYS: &[(f32, f32)] = &[(0.0, 132.5), (0.33, 150.0), (0.66, 115.0), (1.0, 132.5)];
const X_DUR: f32 = 8.0;

const Y1_KEYS: &[(f32, f32)] = &[(0.0, 7.0), (0.4, -25.0), (0.75, 22.0), (1.0, 7.0)];
const Y2_KEYS: &[(f32, f32)] = &[(0.0, 303.0), (0.35, 278.0), (0.7, 325.0), (1.0, 303.0)];
const Y_DUR: f32 = 7.0;

const OFFSET_KEYS: &[(f32, f32)] = &[
    (0.0, 0.25),
    (0.25, 0.85),
    (0.5, 0.45),
    (0.75, 0.7),
    (1.0, 0.25),
];
const OFFSET_DUR: f32 = 6.0;

const STOP1_KEYS: &[(f32, [u8; 3])] = &[
    (0.0, [0x87, 0x5F, 0xFF]),
    (0.25, [0xA0, 0x6A, 0xFF]),
    (0.5, [0x6A, 0x4F, 0xE5]),
    (0.75, [0x9D, 0x7B, 0xFF]),
    (1.0, [0x87, 0x5F, 0xFF]),
];
const STOP1_DUR: f32 = 9.0;

const STOP2_KEYS: &[(f32, [u8; 3])] = &[
    (0.0, [0x00, 0xC8, 0xB3]),
    (0.3, [0x3F, 0xE5, 0xC7]),
    (0.55, [0x00, 0xB3, 0xD0]),
    (0.8, [0x5F, 0xFF, 0xE0]),
    (1.0, [0x00, 0xC8, 0xB3]),
];
const STOP2_DUR: f32 = 11.0;

static FRAME_HANDLES: OnceLock<Vec<svg::Handle>> = OnceLock::new();

pub fn view<'a, Message: 'a>(size: f32, phase: f32) -> Element<'a, Message> {
    let handles = frame_handles();
    let frame_idx = ((phase * FRAMES as f32) as usize) % FRAMES;

    Svg::new(handles[frame_idx].clone())
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .into()
}

fn frame_handles() -> &'static [svg::Handle] {
    FRAME_HANDLES.get_or_init(|| {
        let stripped = strip_animate_tags(ORIGINAL);
        (0..FRAMES)
            .map(|i| {
                let t = i as f32 / FRAMES as f32 * MASTER_CYCLE_SECS;
                let bytes = render_frame(&stripped, t).into_bytes();
                svg::Handle::from_memory(bytes)
            })
            .collect()
    })
}

fn render_frame(stripped: &str, t: f32) -> String {
    let x1 = lerp_scalar(X1_KEYS, (t / X_DUR) % 1.0);
    let x2 = lerp_scalar(X2_KEYS, (t / X_DUR) % 1.0);
    let y1 = lerp_scalar(Y1_KEYS, (t / Y_DUR) % 1.0);
    let y2 = lerp_scalar(Y2_KEYS, (t / Y_DUR) % 1.0);
    let off1 = lerp_scalar(OFFSET_KEYS, (t / OFFSET_DUR) % 1.0);
    let col1 = lerp_color(STOP1_KEYS, (t / STOP1_DUR) % 1.0);
    let col2 = lerp_color(STOP2_KEYS, (t / STOP2_DUR) % 1.0);

    stripped
        .replacen(r#"x1="132.5""#, &format!(r#"x1="{:.2}""#, x1), 1)
        .replacen(r#"x2="132.5""#, &format!(r#"x2="{:.2}""#, x2), 1)
        .replacen(r#"y1="7""#, &format!(r#"y1="{:.2}""#, y1), 1)
        .replacen(r#"y2="303""#, &format!(r#"y2="{:.2}""#, y2), 1)
        .replacen(
            r#"offset="0.721154""#,
            &format!(r#"offset="{:.4}""#, off1),
            1,
        )
        .replacen(
            r##"stop-color="#875FFF""##,
            &format!(r##"stop-color="{}""##, hex(col1)),
            1,
        )
        .replacen(
            r##"stop-color="#00C8B3""##,
            &format!(r##"stop-color="{}""##, hex(col2)),
            1,
        )
}

fn strip_animate_tags(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(start) = rest.find("<animate") {
        out.push_str(&rest[..start]);
        let after = &rest[start..];
        if let Some(self_close) = after.find("/>") {
            let close_tag = after.find("</animate>");
            match close_tag {
                Some(c) if c < self_close => rest = &after[c + "</animate>".len()..],
                _ => rest = &after[self_close + 2..],
            }
        } else if let Some(close) = after.find("</animate>") {
            rest = &after[close + "</animate>".len()..];
        } else {
            out.push_str(after);
            return out;
        }
    }
    out.push_str(rest);
    out
}

fn lerp_scalar(keys: &[(f32, f32)], t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    for w in keys.windows(2) {
        let (t0, v0) = w[0];
        let (t1, v1) = w[1];
        if t <= t1 {
            let alpha = if t1 > t0 { (t - t0) / (t1 - t0) } else { 0.0 };
            return v0 + (v1 - v0) * alpha;
        }
    }
    keys.last().unwrap().1
}

fn lerp_color(keys: &[(f32, [u8; 3])], t: f32) -> [u8; 3] {
    let t = t.clamp(0.0, 1.0);
    for w in keys.windows(2) {
        let (t0, c0) = w[0];
        let (t1, c1) = w[1];
        if t <= t1 {
            let alpha = if t1 > t0 { (t - t0) / (t1 - t0) } else { 0.0 };
            return [
                ((c0[0] as f32) + (c1[0] as f32 - c0[0] as f32) * alpha) as u8,
                ((c0[1] as f32) + (c1[1] as f32 - c0[1] as f32) * alpha) as u8,
                ((c0[2] as f32) + (c1[2] as f32 - c0[2] as f32) * alpha) as u8,
            ];
        }
    }
    keys.last().unwrap().1
}

fn hex(c: [u8; 3]) -> String {
    format!("#{:02X}{:02X}{:02X}", c[0], c[1], c[2])
}
