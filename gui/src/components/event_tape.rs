use cosmic::iced::Length;
use cosmic::widget::{container, scrollable, text, Column, Row};
use cosmic::Element;

use crate::app::{EventEntry, EventKind, Message};

pub fn view<'a, I>(entries: I) -> Element<'a, Message>
where
    I: Iterator<Item = &'a EventEntry>,
{
    let mut col = Column::new()
        .spacing(2)
        .padding(8)
        .width(Length::Fill);
    let mut count = 0;
    for entry in entries {
        col = col.push(row_view(entry));
        count += 1;
    }

    if count == 0 {
        col = col.push(text("Quiet on the wire."));
    }

    container(scrollable(col).width(Length::Fill))
        .height(Length::Fixed(120.0))
        .width(Length::Fill)
        .into()
}

fn row_view<'a>(entry: &'a EventEntry) -> Element<'a, Message> {
    let glyph = match entry.kind {
        EventKind::Attach => "⮕",
        EventKind::Detach => "⮐",
        EventKind::Sleep => "💤",
        EventKind::Wake => "✨",
        EventKind::Kill => "💀",
        EventKind::Spawn => "👻",
        EventKind::Error => "⚠",
    };

    Row::new()
        .push(text(entry.at.format("%H:%M:%S").to_string()).font(cosmic::font::mono()))
        .push(text(glyph))
        .push(text(entry.message.as_str()).width(Length::Fill))
        .spacing(8)
        .width(Length::Fill)
        .into()
}
