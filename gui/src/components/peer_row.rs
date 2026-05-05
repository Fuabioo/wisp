use chrono::{DateTime, Utc};
use cosmic::iced::Length;
use cosmic::widget::{button, text, Row};
use cosmic::Element;

use crate::app::Message;
use crate::backend::PeerInfo;
use crate::components::util;

pub fn view<'a>(session_id: &str, peer: &'a PeerInfo, now: DateTime<Utc>) -> Element<'a, Message> {
    let attached = util::humanize_duration(now.signed_duration_since(peer.connected_at));
    let session_id_owned = session_id.to_string();
    let client_id_owned = peer.client_id.clone();

    Row::new()
        .push(
            text(peer.client_id.as_str())
                .font(cosmic::font::mono())
                .width(Length::FillPortion(2)),
        )
        .push(
            text(format!("{}×{}", peer.width, peer.height))
                .font(cosmic::font::mono())
                .width(Length::FillPortion(1)),
        )
        .push(
            text(peer.remote_addr.as_str())
                .font(cosmic::font::mono())
                .width(Length::FillPortion(2)),
        )
        .push(
            text(attached)
                .font(cosmic::font::mono())
                .width(Length::FillPortion(1)),
        )
        .push(
            button::destructive("kick")
                .on_press(Message::KickPeer(session_id_owned, client_id_owned)),
        )
        .spacing(12)
        .padding(6)
        .into()
}
