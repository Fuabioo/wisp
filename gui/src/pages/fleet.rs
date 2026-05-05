// Fleet — the default view. Single-window layout: rail (left), detail
// spine (right), event tape (bottom). Spawn drawer overlays the spine when
// open. See the wireframe in the design plan.

use cosmic::iced::{Alignment, Length};
use cosmic::widget::table;
use cosmic::widget::{button, container, scrollable, text, text_input, Column, Row};
use cosmic::Element;

use crate::app::{Message, WispAdmin};
use crate::backend::ServerInfo;
use crate::components::{event_tape, ghost_art, session_orb};
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
    let peer_count = app.peer_count(&session.id);
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

    let Some(peer_model) = app.peer_models.get(&session.id) else {
        // Should not happen — peers > 0 but model not yet built. Fall back
        // to the empty card while the next poll fills it in.
        return container(text("(loading peers…)")).padding(8).into();
    };

    let session_id_click = session.id.clone();
    let session_id_dbl = session.id.clone();
    let session_id_sort = session.id.clone();
    let entity_map_dbl = peer_model.entity_to_client.clone();

    let table_widget: Element<'a, Message> = table::table(&peer_model.model)
        .on_item_left_click(move |entity| Message::SelectPeer(session_id_click.clone(), entity))
        .on_item_double_click(move |entity| {
            let client_id = entity_map_dbl.get(&entity).cloned().unwrap_or_default();
            Message::KickPeer(session_id_dbl.clone(), client_id)
        })
        .on_category_left_click(move |cat| Message::SortPeers(session_id_sort.clone(), cat))
        .width(Length::Fill)
        .into();

    let kick_action: Element<'a, Message> = match app.selected_peer_client_id(&session.id) {
        Some(client_id) => button::destructive(format!("kick {}", client_id))
            .on_press(Message::KickPeer(session.id.clone(), client_id))
            .into(),
        None => text("(click a row to select; double-click to kick)")
            .class(theme::ROSE)
            .into(),
    };

    let header_row = Row::new()
        .push(text(format!("peers · {}", peer_count)).size(14))
        .push(container(text("")).width(Length::Fill))
        .push(kick_action)
        .spacing(8)
        .align_y(Alignment::Center);

    container(
        Column::new()
            .push(header_row)
            .push(table_widget)
            .spacing(8)
            .padding(8),
    )
    .class(cosmic::style::Container::Card)
    .width(Length::Fill)
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
