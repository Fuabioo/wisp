// Settings page — edits a `Settings` draft and persists on Save.

use cosmic::iced::{Alignment, Length};
use cosmic::widget::{
    button, container, slider, text, text_input, toggler, Column, Row,
};
use cosmic::Element;

use crate::app::{Message, WispAdmin};
use crate::settings::HamburgerSide;

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

    let decorations_row = Column::new()
        .push(text("Show window decorations").size(14))
        .push(text(
            "When off, the OS / cosmic-shell header bar (title, close \
             button, nav-bar toggle) is hidden for a leaner chrome. The \
             sidebar can still be toggled with Ctrl+B and the right-click \
             menu offers a shortcut to this Settings page.",
        ))
        .push(
            Row::new()
                .push(
                    toggler(app.settings_draft.show_decorations)
                        .on_toggle(Message::SettingsDecorationsChanged),
                )
                .push(text("Show decorations"))
                .spacing(8)
                .align_y(Alignment::Center),
        )
        .spacing(4);

    let alpha_pct = (app.settings_draft.background_alpha * 100.0).round() as u32;
    let alpha_row = Column::new()
        .push(text("Background opacity").size(14))
        .push(text(
            "0% = fully transparent (compositor blur shows through if \
             supported), 100% = solid Catppuccin-Mocha base. Saves to \
             settings on apply.",
        ))
        .push(
            Row::new()
                .push(
                    slider(0.0..=1.0, app.settings_draft.background_alpha, |v| {
                        Message::SettingsAlphaChanged(v)
                    })
                    .step(0.05_f32)
                    .width(Length::Fill),
                )
                .push(
                    text(format!("{}%", alpha_pct))
                        .font(cosmic::font::mono())
                        .width(Length::Fixed(56.0)),
                )
                .spacing(12)
                .align_y(Alignment::Center),
        )
        .spacing(4);

    let blur_row = Column::new()
        .push(text("Enable compositor blur").size(14))
        .push(text(
            "Asks the compositor to blur whatever shows through the \
             transparent surface (wallpaper, windows behind). The \
             opacity slider above controls transparency on its own — \
             blur is just an additive effect. Requires \
             `ext_background_effect_v1` (cosmic-comp ships it; some \
             builds don't bind the global yet, in which case this \
             toggle is a no-op).",
        ))
        .push(
            Row::new()
                .push(
                    toggler(app.settings_draft.enable_blur)
                        .on_toggle(Message::SettingsBlurChanged),
                )
                .push(text("Blur compositor surface behind window"))
                .spacing(8)
                .align_y(Alignment::Center),
        )
        .spacing(4);

    let hamburger_row = Column::new()
        .push(text("Hamburger menu side").size(14))
        .push(text(
            "Which side of the daemon ribbon the menu trigger lives on. \
             Useful when running with decorations off — the menu is the \
             backup access path to nav and Settings.",
        ))
        .push(
            Row::new()
                .push(side_button(
                    "Left",
                    app.settings_draft.hamburger_side == HamburgerSide::Left,
                    HamburgerSide::Left,
                ))
                .push(side_button(
                    "Right",
                    app.settings_draft.hamburger_side == HamburgerSide::Right,
                    HamburgerSide::Right,
                ))
                .spacing(8)
                .align_y(Alignment::Center),
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
            .push(decorations_row)
            .push(alpha_row)
            .push(blur_row)
            .push(hamburger_row)
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

fn side_button<'a>(
    label: &'static str,
    selected: bool,
    side: HamburgerSide,
) -> Element<'a, Message> {
    let btn = if selected {
        button::suggested(label)
    } else {
        button::standard(label)
    };
    btn.on_press(Message::SettingsHamburgerSideChanged(side)).into()
}
