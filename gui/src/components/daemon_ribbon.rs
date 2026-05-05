use cosmic::iced::Length;
use cosmic::widget::{container, text, Row};
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

    container(
        Row::new()
            .push(text("●").class(dot_color))
            .push(text(format!(" {}", status)))
            .push(text("  ·  "))
            .push(text(stats).font(cosmic::font::mono()))
            .spacing(0)
            .padding(8),
    )
    .style(theme::ribbon_style)
    .width(Length::Fill)
    .into()
}
