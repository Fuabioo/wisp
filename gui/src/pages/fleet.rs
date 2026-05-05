// Fleet — the default view. Single-window layout: rail (left), detail
// spine (right), event tape (bottom). Spawn drawer overlays the spine when
// open. See the wireframe in the design plan.

use cosmic::iced::{Alignment, Background, Border, Color, Length};
use cosmic::widget::{
    button, container, mouse_area, scrollable, text, text_input, Column, Row,
};
use cosmic::Element;

use crate::app::{Message, WispAdmin};
use crate::backend::{PeerInfo, ServerInfo};
use crate::components::peers_table::{self, PeerCategory};
use crate::components::{event_tape, ghost_art, util};
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
    let mut col = Column::new().spacing(6).padding(12).push(
        button::suggested("+ spawn session")
            .on_press(Message::OpenSpawnDrawer)
            .width(Length::Fill),
    );

    if app.sessions.is_empty() {
        col = col.push(text("No sessions yet."));
    } else {
        col = col.push(text("").size(4));
        for session in &app.sessions {
            let is_selected = app.selected.as_ref() == Some(&session.id);
            let is_hovered = app.hovered_session.as_ref() == Some(&session.id);
            col = col.push(session_rail_item(
                session,
                app.peer_count(&session.id),
                is_selected,
                is_hovered,
            ));
        }
    }

    container(scrollable(col))
        .class(cosmic::style::Container::Background)
        .height(Length::Fill)
        .width(Length::Fill)
        .into()
}

fn session_rail_item<'a>(
    session: &'a ServerInfo,
    peer_count: usize,
    selected: bool,
    hovered: bool,
) -> Element<'a, Message> {
    let dot = if session.is_active() { "●" } else { "○" };
    let dot_color = if session.is_active() {
        theme::ALIVE
    } else {
        theme::ROSE
    };
    let pip_cluster: String = "◉".repeat(peer_count.min(8));

    let header = Row::new()
        .push(text(dot).class(dot_color))
        .push(text(format!(" {}", session.short_id())).font(cosmic::font::mono()))
        .push(text(format!("  :{}", session.port)).font(cosmic::font::mono()))
        .spacing(0);

    let sub = Row::new()
        .push(text(session.status.label()).class(theme::status_color(session.is_active())))
        .push(text("  ·  "))
        .push(text(format!(
            "{} peer{}",
            peer_count,
            if peer_count == 1 { "" } else { "s" }
        )))
        .push(text(format!("  {}", pip_cluster)))
        .spacing(0);

    let inner = Column::new()
        .push(header)
        .push(sub)
        .spacing(2)
        .padding(8)
        .width(Length::Fill);

    let styled = container(inner)
        .style(row_style(selected, hovered))
        .width(Length::Fill);

    let id_press = session.id.clone();
    let id_enter = session.id.clone();
    let id_exit = session.id.clone();
    mouse_area(styled)
        .on_press(Message::SelectSession(id_press))
        .on_enter(Message::SessionHoverEnter(id_enter))
        .on_exit(Message::SessionHoverExit(id_exit))
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
        .push(header_view(app, session))
        .push(connect_view(app, session))
        .push(peers_view(app, session))
        .spacing(16)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn header_view<'a>(app: &'a WispAdmin, session: &'a ServerInfo) -> Element<'a, Message> {
    // Real icons via cosmic::widget::icon::from_name (resolves against
    // the user's freedesktop icon theme). symbolic variants render
    // monochrome and pick up cosmic's accent / status tinting on hover.
    let icon_btn = |name: &'static str, msg: Message| -> Element<'a, Message> {
        cosmic::widget::button::icon(cosmic::widget::icon::from_name(name).handle())
            .on_press(msg)
            .extra_small()
            .into()
    };

    let action_area: Element<'a, Message> = if app.kill_confirm.as_ref() == Some(&session.id) {
        Row::new()
            .push(text("kill?"))
            .push(icon_btn("window-close-symbolic", Message::CancelKill))
            .push(icon_btn(
                "object-select-symbolic",
                Message::ConfirmKill(session.id.clone()),
            ))
            .spacing(6)
            .align_y(Alignment::Center)
            .into()
    } else {
        let toggle: Element<'a, Message> = if session.is_active() {
            icon_btn(
                "media-playback-pause-symbolic",
                Message::DownSession(session.id.clone()),
            )
        } else {
            icon_btn(
                "media-playback-start-symbolic",
                Message::UpSession(session.id.clone()),
            )
        };
        Row::new()
            .push(toggle)
            .push(icon_btn(
                "view-refresh-symbolic",
                Message::RefreshSession(session.id.clone()),
            ))
            .push(icon_btn(
                "edit-delete-symbolic",
                Message::AskKill(session.id.clone()),
            ))
            .spacing(6)
            .align_y(Alignment::Center)
            .into()
    };

    let top_row = Row::new()
        .push(ghost_art::view::<Message>(96.0, app.anim_phase))
        .push(container(text("")).width(Length::Fill))
        .push(action_area)
        .align_y(Alignment::Center);

    Column::new()
        .push(top_row)
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
    let hovered_client = app
        .hovered_peer
        .as_ref()
        .filter(|(s, _)| s == &session.id)
        .map(|(_, c)| c);

    let header_row = Row::new()
        .push(sort_header(PeerCategory::Client, &session.id, sort))
        .push(sort_header(PeerCategory::Window, &session.id, sort))
        .push(sort_header(PeerCategory::Remote, &session.id, sort))
        .push(sort_header(PeerCategory::Attached, &session.id, sort))
        .spacing(0);

    let now = chrono::Utc::now();
    let mut rows = Column::new().spacing(2);
    for peer in sorted {
        let is_selected = selected_client == Some(&peer.client_id);
        let is_hovered = hovered_client == Some(&peer.client_id);
        rows = rows.push(peer_table_row(&session.id, peer, now, is_selected, is_hovered));
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
            .push(rows)
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

fn peer_table_row<'a>(
    session_id: &'a str,
    peer: &'a PeerInfo,
    now: chrono::DateTime<chrono::Utc>,
    selected: bool,
    hovered: bool,
) -> Element<'a, Message> {
    let inner = Row::new()
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
        .padding(6);

    let styled = container(inner)
        .style(row_style(selected, hovered))
        .width(Length::Fill);

    let s_press = session_id.to_string();
    let c_press = peer.client_id.clone();
    let s_enter = session_id.to_string();
    let c_enter = peer.client_id.clone();
    let s_exit = session_id.to_string();
    let c_exit = peer.client_id.clone();
    mouse_area(styled)
        .on_press(Message::SelectPeer(s_press, c_press))
        .on_enter(Message::PeerHoverEnter(s_enter, c_enter))
        .on_exit(Message::PeerHoverExit(s_exit, c_exit))
        .into()
}

/// Hover-aware container style for selectable list rows. WISP-tinted
/// background — strong when selected, light on hover, transparent at rest.
fn row_style(
    selected: bool,
    hovered: bool,
) -> impl Fn(&cosmic::Theme) -> container::Style + 'static {
    move |_theme| {
        let alpha = if selected {
            0.28
        } else if hovered {
            0.10
        } else {
            0.0
        };
        let mut style = container::Style::default();
        if alpha > 0.0 {
            style.background = Some(Background::Color(Color {
                r: theme::WISP.r,
                g: theme::WISP.g,
                b: theme::WISP.b,
                a: alpha,
            }));
        }
        style.border = Border {
            radius: 8.0.into(),
            ..Default::default()
        };
        style
    }
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
