// Pixel ghost — the brand leitmotif. Loads `assets/logo-wisp.svg` directly
// so the GUI's chrome stays anchored to the canonical wisp logo (see ADR
// 0002's brand-DNA contract).

use cosmic::iced::Length;
use cosmic::widget::svg::{self, Svg};
use cosmic::Element;

const LOGO_BYTES: &[u8] = include_bytes!("../../../assets/logo-wisp.svg");

pub fn view<'a, Message: 'a>(size: f32) -> Element<'a, Message> {
    Svg::new(svg::Handle::from_memory(LOGO_BYTES))
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .into()
}
