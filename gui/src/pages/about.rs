// About page — version, ghost, tribute.

use cosmic::iced::{Alignment, Length};
use cosmic::widget::{container, text, Column};
use cosmic::Element;

use crate::app::{Message, WispAdmin};
use crate::components::ghost_art;

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn view<'a>(_app: &'a WispAdmin) -> Element<'a, Message> {
    container(
        Column::new()
            .push(ghost_art::view::<Message>())
            .push(text("wisp-admin").size(28).font(cosmic::font::mono()))
            .push(text(format!("v{}", VERSION)).font(cosmic::font::mono()))
            .push(text(""))
            .push(text("COSMIC-native admin GUI for the wisp daemon."))
            .push(text(
                "Made with charm — wish, lipgloss, bubbletea — and libcosmic.",
            ))
            .push(text(""))
            .push(text(
                "brand DNA: pet.txt + lipgloss 99/212/204 (see ADR 0002).",
            ))
            .spacing(8)
            .align_x(Alignment::Center)
            .padding(48),
    )
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
