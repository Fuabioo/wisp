use cosmic::iced::{Alignment, Length};
use cosmic::widget::{button, container, text, Row};
use cosmic::Element;

use crate::app::{Message, WispAdmin};
use crate::components::util;
use crate::theme;

pub fn view<'a>(app: &'a WispAdmin) -> Element<'a, Message> {
    let dot_color = if app.daemon_reachable {
        theme::ALIVE
    } else {
        theme::DANGER
    };
    let status = if app.daemon_reachable {
        "daemon · reachable"
    } else {
        "daemon · unreachable"
    };

    let uptime = match app.daemon_started_at {
        Some(start) => util::humanize_duration(chrono::Utc::now().signed_duration_since(start)),
        None => "—".to_string(),
    };

    let total_peers: usize = app.peers.values().map(|v| v.len()).sum();

    let stats = format!(
        "{} sessions · {} peers · uptime {}",
        app.sessions.len(),
        total_peers,
        uptime
    );

    let menu_btn: Element<'a, Message> = if app.menu_open {
        button::custom(text("≡").size(18))
            .padding(6)
            .class(cosmic::theme::Button::Suggested)
            .width(Length::Fixed(36.0))
            .height(Length::Fixed(36.0))
            .on_press(Message::CloseMenu)
            .into()
    } else {
        button::custom(text("≡").size(18))
            .padding(6)
            .class(cosmic::theme::Button::Icon)
            .width(Length::Fixed(36.0))
            .height(Length::Fixed(36.0))
            .on_press(Message::OpenMenu)
            .into()
    };

    container(
        Row::new()
            .push(text("●").class(dot_color))
            .push(text(format!(" {}", status)))
            .push(text("  ·  "))
            .push(text(stats).font(cosmic::font::mono()))
            .push(container(text("")).width(Length::Fill))
            .push(menu_btn)
            .spacing(0)
            .padding([4, 8])
            .align_y(Alignment::Center),
    )
    .style(theme::ribbon_style)
    .width(Length::Fill)
    .into()
}
