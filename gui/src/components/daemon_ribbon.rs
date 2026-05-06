use cosmic::iced::{Alignment, Length};
use cosmic::widget::{container, text, Row};
use cosmic::Element;

use crate::app::{Message, WispAdmin};
use crate::components::util;
use crate::settings::HamburgerSide;
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

    let menu_handle = cosmic::widget::icon::from_name("open-menu-symbolic").handle();
    let menu_btn: Element<'a, Message> = if app.menu_open {
        cosmic::widget::button::icon(menu_handle)
            .on_press(Message::CloseMenu)
            .extra_small()
            .selected(true)
            .into()
    } else {
        cosmic::widget::button::icon(menu_handle)
            .on_press(Message::OpenMenu)
            .extra_small()
            .into()
    };

    let info_row = Row::new()
        .push(text("●").class(dot_color))
        .push(text(format!(" {}", status)))
        .push(text("  ·  "))
        .push(text(stats).font(cosmic::font::mono()))
        .spacing(0)
        .align_y(Alignment::Center);

    let row = match app.settings.hamburger_side {
        HamburgerSide::Left => Row::new()
            .push(menu_btn)
            .push(info_row)
            .push(container(text("")).width(Length::Fill)),
        HamburgerSide::Right => Row::new()
            .push(info_row)
            .push(container(text("")).width(Length::Fill))
            .push(menu_btn),
    };

    container(row.spacing(8).padding([4, 8]).align_y(Alignment::Center))
        .style(theme::ribbon_style)
        .width(Length::Fill)
        .into()
}
