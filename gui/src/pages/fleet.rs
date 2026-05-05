// Fleet — the default view. Single-window layout: rail (left), detail
// spine (right), event tape (bottom). Spawn drawer overlays the spine when
// open. See the wireframe in the design plan.

use cosmic::iced::{Alignment, Length};
use cosmic::widget::{button, container, scrollable, text, text_input, Column, Row};
use cosmic::Element;

use crate::app::{Message, WispAdmin};
use crate::backend::ServerInfo;
use crate::components::{event_tape, ghost_art, peer_row, session_orb};
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
        .push(connect_view(session))
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

fn connect_view<'a>(session: &'a ServerInfo) -> Element<'a, Message> {
    container(
        Row::new()
            .push(text(format!("ssh -p {} localhost", session.port)).font(cosmic::font::mono()))
            .spacing(8)
            .padding(8)
            .align_y(Alignment::Center),
    )
    .class(cosmic::style::Container::Card)
    .width(Length::Fill)
    .into()
}

fn peers_view<'a>(app: &'a WispAdmin, session: &'a ServerInfo) -> Element<'a, Message> {
    match app.peers.get(&session.id) {
        Some(peers) if !peers.is_empty() => {
            let now = chrono::Utc::now();
            let mut col = Column::new()
                .spacing(4)
                .padding(4)
                .push(text("peers").size(14));
            for peer in peers {
                col = col.push(peer_row::view(&session.id, peer, now));
            }
            container(col)
                .class(cosmic::style::Container::Card)
                .width(Length::Fill)
                .into()
        }
        _ => container(
            Column::new()
                .push(text("(no peers attached) 👻"))
                .padding(8),
        )
        .class(cosmic::style::Container::Card)
        .width(Length::Fill)
        .into(),
    }
}

fn actions_view<'a>(app: &'a WispAdmin, session: &'a ServerInfo) -> Element<'a, Message> {
    if app.kill_confirm.as_ref() == Some(&session.id) {
        return Row::new()
            .push(text("Are you sure? The shell dies on kill."))
            .push(button::standard("cancel").on_press(Message::CancelKill))
            .push(button::destructive("kill").on_press(Message::ConfirmKill(session.id.clone())))
            .spacing(12)
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
        .push(button::destructive("kill").on_press(Message::AskKill(session.id.clone())))
        .spacing(12)
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
