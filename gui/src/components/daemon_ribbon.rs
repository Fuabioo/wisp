use cosmic::iced::Length;
use cosmic::widget::{container, text, Row};
use cosmic::Element;

use crate::app::{Message, WispAdmin};
use crate::components::util;

pub fn view<'a>(app: &'a WispAdmin) -> Element<'a, Message> {
    let heartbeat = if app.daemon_reachable { "● " } else { "○ " };
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

    container(
        Row::new()
            .push(text(heartbeat))
            .push(text(status))
            .push(text("  ·  "))
            .push(text(stats).font(cosmic::font::mono()))
            .spacing(0)
            .padding(8),
    )
    .class(cosmic::style::Container::Background)
    .width(Length::Fill)
    .into()
}
