// Settings page — edits a `Settings` draft and persists on Save.

use cosmic::iced::{Alignment, Length};
use cosmic::widget::{button, container, text, text_input, Column, Row};
use cosmic::Element;

use crate::app::{Message, WispAdmin};

pub fn view<'a>(app: &'a WispAdmin) -> Element<'a, Message> {
    let dirty = app.settings_draft != app.settings;

    let header = Column::new()
        .push(text("settings").size(24).font(cosmic::font::mono()))
        .push(text(
            "Persisted to $XDG_CONFIG_HOME/wisp-admin/settings.toml — \
             applied to subsequent spawns.",
        ))
        .spacing(6);

    let shell_row = Column::new()
        .push(text("Default shell").size(14))
        .push(text(
            "Path to the shell binary used by `wisp server`. Leave blank to \
             fall back to the daemon's $SHELL detection (zsh if unset).",
        ))
        .push(
            text_input("/usr/bin/zsh", &app.settings_draft.default_shell)
                .on_input(Message::SettingsShellChanged),
        )
        .spacing(4);

    let host_row = Column::new()
        .push(text("Connect host").size(14))
        .push(text(
            "Shown in the session detail's `ssh -p PORT <host>` snippet. \
             Defaults to the system hostname so peers on the LAN can copy \
             the string verbatim.",
        ))
        .push(
            text_input("hostname or IP", &app.settings_draft.connect_host)
                .on_input(Message::SettingsHostChanged),
        )
        .spacing(4);

    let save_btn: Element<'a, Message> = if dirty {
        button::suggested("Save").on_press(Message::SaveSettings).into()
    } else {
        button::standard("Save").into()
    };

    let revert_btn: Element<'a, Message> = if dirty {
        button::standard("Revert").on_press(Message::RevertSettings).into()
    } else {
        button::standard("Revert").into()
    };

    let actions = Row::new()
        .push(save_btn)
        .push(revert_btn)
        .push(button::standard("Reset to system defaults").on_press(Message::ResetSettings))
        .spacing(8)
        .align_y(Alignment::Center);

    container(
        Column::new()
            .push(header)
            .push(shell_row)
            .push(host_row)
            .push(actions)
            .spacing(20)
            .padding(24)
            .max_width(640.0),
    )
    .center_x(Length::Fill)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
