use cosmic::iced::Length;
use cosmic::widget::button;
use cosmic::Element;

use crate::app::Message;
use crate::backend::ServerInfo;

pub fn view<'a>(
    session: &'a ServerInfo,
    peer_count: usize,
    selected: bool,
) -> Element<'a, Message> {
    let dot = if session.is_active() { "●" } else { "○" };
    let pip_cluster: String = "◉".repeat(peer_count.min(8));
    let label = format!(
        "{}  {}  :{}   {}   {}{}  {}",
        dot,
        session.short_id(),
        session.port,
        session.status.label(),
        peer_count,
        if peer_count == 1 { " peer" } else { " peers" },
        pip_cluster,
    );

    let click = Message::SelectSession(session.id.clone());
    if selected {
        button::suggested(label)
            .on_press(click)
            .width(Length::Fill)
            .into()
    } else {
        button::standard(label)
            .on_press(click)
            .width(Length::Fill)
            .into()
    }
}
