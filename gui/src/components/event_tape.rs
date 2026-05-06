use cosmic::iced::{Alignment, Length};
use cosmic::widget::{button, container, scrollable, text, Column, Row};
use cosmic::Element;

use crate::app::{EventEntry, EventKind, Message, EVENT_TAPE_SCROLL};

pub fn view<'a, I>(entries: I, following: bool) -> Element<'a, Message>
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

    let scroll: Element<'a, Message> = scrollable(col)
        .id(EVENT_TAPE_SCROLL.clone())
        .on_scroll(Message::EventTapeScrolled)
        .width(Length::Fill)
        .height(Length::Fill)
        .into();

    // Floating "follow logs ↓" button overlays the bottom-right of
    // the tape when the user has scrolled away from the latest entry.
    // Stays out of the layout flow so the rest of the tape doesn't
    // shift around when it appears / disappears.
    let body: Element<'a, Message> = if following {
        scroll
    } else {
        let btn: Element<'a, Message> = button::suggested("follow logs  ↓")
            .on_press(Message::EventTapeFollow)
            .into();
        Column::new()
            .push(scroll)
            .push(
                Row::new()
                    .push(container(text("")).width(Length::Fill))
                    .push(btn)
                    .padding([0, 12, 8, 0])
                    .align_y(Alignment::Center),
            )
            .height(Length::Fill)
            .width(Length::Fill)
            .into()
    };

    container(body)
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
