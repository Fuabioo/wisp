// Fleet — the default view. Single-window layout: rail (left), detail
// spine (right), event tape (bottom). Spawn drawer overlays the spine when
// open. See the wireframe in the design plan.

use cosmic::iced::{Alignment, Length};
use cosmic::widget::{button, container, list, scrollable, text, text_input, Column, Row};
use cosmic::Element;

use crate::app::{Message, WispAdmin};
use crate::backend::{PeerInfo, ServerInfo};
use crate::components::peers_table::{self, PeerCategory};
use crate::components::{event_tape, ghost_art, session_orb, util};
use crate::theme;

pub fn view<'a>(app: &'a WispAdmin) -> Element<'a, Message> {
    let rail = rail_view(app);
    let spine = spine_view(app);
    let tape = event_tape::view(app.event_tape.iter().rev().take(40).rev());

    Column::new()
        .push(
            Row::new()
                .push(
                    container(rail)
                        .width(Length::Fixed(260.0))
                        .height(Length::Fill),
                )
                .push(
                    container(spine)
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .padding(16),
                )
                .height(Length::Fill)
                .width(Length::Fill),
        )
        .push(tape)
        .height(Length::Fill)
        .width(Length::Fill)
        .into()
}

fn rail_view<'a>(app: &'a WispAdmin) -> Element<'a, Message> {
    let mut col = Column::new().spacing(8).padding(12).push(
        button::suggested("+ spawn session").on_press(Message::OpenSpawnDrawer),
    );

    if app.sessions.is_empty() {
        col = col.push(text("No sessions yet."));
    } else {
        for session in &app.sessions {
            let is_selected = app.selected.as_ref() == Some(&session.id);
            col = col.push(session_orb::view(
                session,
                app.peer_count(&session.id),
                is_selected,
            ));
        }
    }

    container(scrollable(col))
        .class(cosmic::style::Container::Background)
        .height(Length::Fill)
        .width(Length::Fill)
        .into()
}

fn spine_view<'a>(app: &'a WispAdmin) -> Element<'a, Message> {
    if app.spawn_drawer.open {
        return spawn_drawer_view(app);
    }

    let Some(selected_id) = &app.selected else {
        return empty_state_view(app.anim_phase);
    };
    let Some(session) = app.sessions.iter().find(|s| &s.id == selected_id) else {
        return empty_state_view(app.anim_phase);
    };

    Column::new()
        .push(header_view(session, app.anim_phase))
        .push(connect_view(app, session))
        .push(peers_view(app, session))
        .push(actions_view(app, session))
        .spacing(16)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn header_view<'a>(session: &'a ServerInfo, anim_phase: f32) -> Element<'a, Message> {
    Column::new()
        .push(ghost_art::view::<Message>(96.0, anim_phase))
        .push(
            text(format!("session · {}", session.short_id()))
                .size(20)
                .font(cosmic::font::mono()),
        )
        .push(
            Row::new()
                .push(text(format!("port :{}", session.port)).font(cosmic::font::mono()))
                .push(text("  ·  "))
                .push(
                    text(session.status.label())
                        .class(theme::status_color(session.is_active())),
                )
                .spacing(0),
        )
        .spacing(6)
        .into()
}

fn connect_view<'a>(app: &'a WispAdmin, session: &'a ServerInfo) -> Element<'a, Message> {
    let host = if app.settings.connect_host.is_empty() {
        "localhost"
    } else {
        app.settings.connect_host.as_str()
    };
    container(
        Row::new()
            .push(text(format!("ssh -p {} {}", session.port, host)).font(cosmic::font::mono()))
            .spacing(8)
            .padding(8)
            .align_y(Alignment::Center),
    )
    .class(cosmic::style::Container::Card)
    .width(Length::Fill)
    .into()
}

fn peers_view<'a>(app: &'a WispAdmin, session: &'a ServerInfo) -> Element<'a, Message> {
    let peers = app.peers.get(&session.id);
    let peer_count = peers.map(Vec::len).unwrap_or(0);
    if peer_count == 0 {
        return container(
            Column::new()
                .push(text("peers").size(14))
                .push(text("(no peers attached) 👻"))
                .spacing(4)
                .padding(8),
        )
        .class(cosmic::style::Container::Card)
        .width(Length::Fill)
        .into();
    }
    let peers = peers.unwrap();

    let sort = app.peer_sorts.get(&session.id).copied();
    let mut sorted: Vec<&PeerInfo> = peers.iter().collect();
    if let Some((category, ascending)) = sort {
        sorted.sort_by(|a, b| {
            let ord = peers_table::compare(a, b, category);
            if ascending {
                ord
            } else {
                ord.reverse()
            }
        });
    }

    let selected_client = app.selected_peers.get(&session.id);

    let header_row = Row::new()
        .push(sort_header(PeerCategory::Client, &session.id, sort))
        .push(sort_header(PeerCategory::Window, &session.id, sort))
        .push(sort_header(PeerCategory::Remote, &session.id, sort))
        .push(sort_header(PeerCategory::Attached, &session.id, sort))
        .spacing(0);

    let now = chrono::Utc::now();
    let mut list = list::list_column().list_item_padding(8);
    for peer in sorted {
        let is_selected = selected_client == Some(&peer.client_id);
        list = list.add(
            list::button(peer_row(peer, now))
                .selected(is_selected)
                .on_press(Message::SelectPeer(
                    session.id.clone(),
                    peer.client_id.clone(),
                )),
        );
    }

    let kick_action: Element<'a, Message> = match selected_client {
        Some(client_id) => button::destructive(format!("kick {}", client_id))
            .on_press(Message::KickPeer(session.id.clone(), client_id.clone()))
            .into(),
        None => text("(click a row to select)").class(theme::ROSE).into(),
    };

    let title_row = Row::new()
        .push(text(format!("peers · {}", peer_count)).size(14))
        .push(container(text("")).width(Length::Fill))
        .push(kick_action)
        .spacing(8)
        .align_y(Alignment::Center);

    container(
        Column::new()
            .push(title_row)
            .push(header_row)
            .push(list.into_element())
            .spacing(8)
            .padding(8),
    )
    .class(cosmic::style::Container::Card)
    .width(Length::Fill)
    .into()
}

fn sort_header<'a>(
    category: PeerCategory,
    session_id: &str,
    current: Option<(PeerCategory, bool)>,
) -> Element<'a, Message> {
    let indicator = match current {
        Some((c, true)) if c == category => " ▲",
        Some((c, false)) if c == category => " ▼",
        _ => "",
    };
    let label = format!("{}{}", category.label(), indicator);
    let session_id = session_id.to_string();
    button::text(label)
        .on_press(Message::SortPeers(session_id, category))
        .width(Length::FillPortion(category.width_portion()))
        .into()
}

fn peer_row<'a>(peer: &'a PeerInfo, now: chrono::DateTime<chrono::Utc>) -> Element<'a, Message> {
    Row::new()
        .push(
            text(peer.client_id.as_str())
                .font(cosmic::font::mono())
                .width(Length::FillPortion(PeerCategory::Client.width_portion())),
        )
        .push(
            text(format!("{}×{}", peer.width, peer.height))
                .font(cosmic::font::mono())
                .width(Length::FillPortion(PeerCategory::Window.width_portion())),
        )
        .push(
            text(peer.remote_addr.as_str())
                .font(cosmic::font::mono())
                .width(Length::FillPortion(PeerCategory::Remote.width_portion())),
        )
        .push(
            text(util::humanize_duration(
                now.signed_duration_since(peer.connected_at),
            ))
            .font(cosmic::font::mono())
            .width(Length::FillPortion(PeerCategory::Attached.width_portion())),
        )
        .spacing(12)
        .align_y(Alignment::Center)
        .into()
}

fn actions_view<'a>(app: &'a WispAdmin, session: &'a ServerInfo) -> Element<'a, Message> {
    if app.kill_confirm.as_ref() == Some(&session.id) {
        return Row::new()
            .push(text("Are you sure? The shell dies on kill."))
            .push(button::standard("cancel").on_press(Message::CancelKill))
            .push(button::destructive("kill").on_press(Message::ConfirmKill(session.id.clone())))
            .spacing(12)
            .align_y(Alignment::Center)
            .into();
    }

    let toggle: Element<'a, Message> = if session.is_active() {
        button::standard("💤 sleep")
            .on_press(Message::DownSession(session.id.clone()))
            .into()
    } else {
        button::suggested("✨ wake")
            .on_press(Message::UpSession(session.id.clone()))
            .into()
    };

    Row::new()
        .push(toggle)
        .push(button::standard("🔁 refresh").on_press(Message::RefreshSession(session.id.clone())))
        .push(button::destructive("kill").on_press(Message::AskKill(session.id.clone())))
        .spacing(12)
        .align_y(Alignment::Center)
        .into()
}

fn spawn_drawer_view<'a>(app: &'a WispAdmin) -> Element<'a, Message> {
    container(
        Column::new()
            .push(text("Spawn a new wisp session").size(20))
            .push(text("Pick a port. Defaults to 2222."))
            .push(
                text_input("port", &app.spawn_drawer.port_input)
                    .on_input(Message::SpawnPortChanged)
                    .on_submit(|_| Message::SpawnSubmit),
            )
            .push(
                Row::new()
                    .push(button::standard("cancel").on_press(Message::CloseSpawnDrawer))
                    .push(button::suggested("spawn").on_press(Message::SpawnSubmit))
                    .spacing(8),
            )
            .spacing(12)
            .padding(24)
            .width(Length::Fill),
    )
    .class(cosmic::style::Container::Card)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn empty_state_view<'a>(anim_phase: f32) -> Element<'a, Message> {
    container(
        Column::new()
            .push(ghost_art::view::<Message>(200.0, anim_phase))
            .push(text("No wisps yet.").size(20))
            .push(text("Summon your first session to begin."))
            .push(button::suggested("+ spawn session").on_press(Message::OpenSpawnDrawer))
            .spacing(12)
            .align_x(Alignment::Center)
            .padding(48),
    )
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
