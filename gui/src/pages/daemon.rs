// Daemon page — health overview. v1 surfaces what we know from polling
// (reachable + uptime + counts). v2 adds socket path, version string, and
// log tail (depends on a daemon-side `wisp logs` RPC; see TODO.md).

use cosmic::iced::Length;
use cosmic::widget::{container, text, Column};
use cosmic::Element;

use crate::app::{Message, WispAdmin};

pub fn view<'a>(app: &'a WispAdmin) -> Element<'a, Message> {
    let reachability = if app.daemon_reachable {
        "● reachable"
    } else {
        "○ unreachable"
    };

    let started = app
        .daemon_started_at
        .map(|t| t.format("%Y-%m-%d %H:%M:%S UTC").to_string())
        .unwrap_or_else(|| "—".to_string());

    let total_peers: usize = app.peers.values().map(|v| v.len()).sum();

    container(
        Column::new()
            .push(text("daemon").size(24).font(cosmic::font::mono()))
            .push(text(reachability))
            .push(text(format!("first reachable: {}", started)).font(cosmic::font::mono()))
            .push(text(format!("sessions: {}", app.sessions.len())).font(cosmic::font::mono()))
            .push(text(format!("attached peers: {}", total_peers)).font(cosmic::font::mono()))
            .push(text(""))
            .push(text(
                "v2 will add socket path, daemon version, and `wisp logs <id>` tail.",
            ))
            .spacing(8)
            .padding(24)
            .width(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
