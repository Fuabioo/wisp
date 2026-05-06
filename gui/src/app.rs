use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};
use std::time::Duration;

use cosmic::app::{Core, Task};
use cosmic::iced::widget::scrollable as iced_scrollable;
use cosmic::iced::{event, keyboard, mouse, window, Length, Point, Subscription};
use cosmic::widget::{button, container, menu, nav_bar, popover, text, Column, Row};
use cosmic::Element;

/// Stable Id for the event-tape scrollable. Used by the "follow
/// logs" plumbing so we can issue programmatic `scroll_to` Tasks.
/// `iced_widget::scrollable::Id` itself is private (re-exported only
/// through use), so we go through `iced::core::widget::Id` which is
/// the public alias and what `scrollable::scroll_to` accepts.
pub static EVENT_TAPE_SCROLL: LazyLock<cosmic::iced::core::widget::Id> =
    LazyLock::new(cosmic::iced::core::widget::Id::unique);

/// Last known cursor position, updated on every CursorMoved event from
/// the iced subscription. Read at view-time when we need to anchor the
/// right-click popover. Living outside the app state means cursor
/// movement doesn't trigger an iced re-render — only right-clicks do.
static CURSOR_POS: Mutex<Option<Point>> = Mutex::new(None);

use crate::backend::{CliBackend, PeerInfo, ServerInfo, WispBackend};
use crate::components::peers_table::PeerCategory;
use crate::settings::Settings;
use crate::theme;

pub struct WispAdmin {
    core: Core,
    pub backend: Arc<dyn WispBackend>,
    pub nav: nav_bar::Model,
    pub sessions: Vec<ServerInfo>,
    pub selected: Option<String>,
    pub peers: HashMap<String, Vec<PeerInfo>>,
    /// Per-session sort preference: (column, ascending).
    pub peer_sorts: HashMap<String, (PeerCategory, bool)>,
    /// Per-session selected client_id (the row that's highlighted).
    pub selected_peers: HashMap<String, String>,
    pub daemon_reachable: bool,
    pub daemon_started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub spawn_drawer: SpawnDrawerState,
    pub kill_confirm: Option<String>,
    pub last_error: Option<String>,
    pub event_tape: Vec<EventEntry>,
    pub window_focused: bool,
    pub anim_phase: f32,
    pub settings: Settings,
    pub settings_draft: Settings,
    pub hovered_session: Option<String>,
    pub hovered_peer: Option<(String, String)>,
    pub menu_open: bool,
    /// Cursor position frozen at the moment the menu opened. Without
    /// this the popover re-anchors on every re-render — the menu would
    /// trail the cursor as the user tried to click an entry.
    pub menu_anchor: Option<Point>,
    /// Whether the event tape is auto-scrolling to the most recent
    /// entry. Toggled to `false` when the user scrolls up away from
    /// the end; the "follow logs" button surfaces while this is
    /// false. Re-enabled either when the user scrolls back to the
    /// bottom or hits the follow button.
    pub event_tape_following: bool,
}

/// Master cycle for the ghost shimmer — chosen to match the longest SMIL
/// period on the source SVG (the teal stop-colour, 11 s) so all
/// sub-animations complete at least one cycle inside it.
/// `ghost_art::view` decomposes this into per-attribute timings.
const ANIM_CYCLE_SECS: f32 = 11.0;
/// Anim heartbeat rate. With a transparent surface every redraw is
/// observable as a flash because iced clears the surface before
/// repainting, so we tick slowly: ANIM_CYCLE_SECS / FRAMES (64) ≈
/// 172 ms is the minimum useful interval. At 320 ms (~3 Hz) one frame
/// in two is duplicated, which is plenty for the slow gradient breathing
/// the ghost performs and keeps re-render flicker out of view.
const ANIM_TICK_MS: u64 = 320;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Fleet,
    Daemon,
    Settings,
    About,
}

impl Page {
    fn label(self) -> &'static str {
        match self {
            Page::Fleet => "Fleet",
            Page::Daemon => "Daemon",
            Page::Settings => "Settings",
            Page::About => "About",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct SpawnDrawerState {
    pub open: bool,
    pub port_input: String,
}

#[derive(Debug, Clone)]
pub struct EventEntry {
    pub at: chrono::DateTime<chrono::Utc>,
    pub kind: EventKind,
    pub message: String,
}

#[derive(Debug, Clone, Copy)]
pub enum EventKind {
    Attach,
    Detach,
    Sleep,
    Wake,
    Kill,
    Spawn,
    Error,
}

#[derive(Debug, Clone)]
pub enum Message {
    Tick,
    AnimTick,
    WindowFocused(bool),
    SelectSession(String),

    SessionsLoaded(Result<Vec<ServerInfo>, String>),
    PeersLoaded(String, Result<Vec<PeerInfo>, String>),

    OpenSpawnDrawer,
    CloseSpawnDrawer,
    SpawnPortChanged(String),
    SpawnSubmit,
    SpawnDone(Result<ServerInfo, String>),

    UpSession(String),
    DownSession(String),
    AskKill(String),
    CancelKill,
    ConfirmKill(String),
    SessionActionDone(SessionAction, String, Result<(), String>),

    SelectPeer(String, String),
    SortPeers(String, PeerCategory),
    KickPeer(String, String),
    KickDone(String, String, Result<(), String>),

    RefreshSession(String),
    RefreshDone(String, Result<(), String>),

    SettingsShellChanged(String),
    SettingsHostChanged(String),
    SettingsDecorationsChanged(bool),
    SettingsAlphaChanged(f32),
    SettingsBlurChanged(bool),
    SettingsHamburgerSideChanged(crate::settings::HamburgerSide),
    SaveSettings,
    RevertSettings,
    ResetSettings,

    NavigateTo(Page),
    ToggleSidebar,
    ApplyInitialSettings,

    SessionHoverEnter(String),
    SessionHoverExit(String),
    PeerHoverEnter(String, String),
    PeerHoverExit(String, String),

    OpenMenu,
    CloseMenu,

    EventTapeScrolled(iced_scrollable::Viewport),
    EventTapeFollow,

    DismissError,
}

#[derive(Debug, Clone, Copy)]
pub enum SessionAction {
    Up,
    Down,
    Kill,
}

/// Lift a user `Message` into cosmic's outer `Action` so it matches what
/// `Task::perform`'s mapper signature expects.
fn act(m: Message) -> cosmic::Action<Message> {
    cosmic::Action::App(m)
}

impl cosmic::Application for WispAdmin {
    type Executor = cosmic::executor::Default;
    type Flags = ();
    type Message = Message;

    const APP_ID: &'static str = "dev.fabiomora.WispAdmin";

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(mut core: Core, _flags: Self::Flags) -> (Self, Task<Self::Message>) {
        tracing::info!(app_id = Self::APP_ID, "WispAdmin init");
        // Critical for transparency: cosmic's framework otherwise wraps
        // our view in an opaque container painted at cosmic.background.
        // base, masking the alpha we set in `Application::style()`. With
        // this off, the body shows the window's clear-color (and the
        // wallpaper / compositor blur behind it). Same flag cosmic-term
        // sets at src/main.rs:1424.
        core.window.content_container = false;
        let backend: Arc<dyn WispBackend> = Arc::new(CliBackend::new());
        let settings = Settings::load();
        tracing::info!(
            shell = %settings.default_shell,
            host = %settings.connect_host,
            "settings loaded"
        );
        let mut nav = nav_bar::Model::default();
        nav.insert()
            .text(Page::Fleet.label())
            .data::<Page>(Page::Fleet)
            .activate();
        nav.insert()
            .text(Page::Daemon.label())
            .data::<Page>(Page::Daemon);
        nav.insert()
            .text(Page::Settings.label())
            .data::<Page>(Page::Settings);
        nav.insert()
            .text(Page::About.label())
            .data::<Page>(Page::About);

        let app = WispAdmin {
            core,
            backend: backend.clone(),
            nav,
            sessions: Vec::new(),
            selected: None,
            peers: HashMap::new(),
            peer_sorts: HashMap::new(),
            selected_peers: HashMap::new(),
            daemon_reachable: false,
            daemon_started_at: None,
            spawn_drawer: SpawnDrawerState::default(),
            kill_confirm: None,
            last_error: None,
            event_tape: Vec::with_capacity(200),
            window_focused: true,
            anim_phase: 0.0,
            settings_draft: settings.clone(),
            settings,
            hovered_session: None,
            hovered_peer: None,
            menu_open: false,
            menu_anchor: None,
            event_tape_following: true,
        };

        let initial_load = Task::perform(
            async move { backend.list_servers().await.map_err(|e| e.to_string()) },
            |result| act(Message::SessionsLoaded(result)),
        );
        // Self-message that fires after the framework has set up the
        // window, so we can apply the persisted decorations + blur state
        // without racing main_window_id().
        let apply_initial = Task::done(act(Message::ApplyInitialSettings));

        (app, Task::batch([initial_load, apply_initial]))
    }

    fn update(&mut self, message: Self::Message) -> Task<Self::Message> {
        match message {
            Message::Tick => {
                tracing::debug!(
                    sessions = self.sessions.len(),
                    selected = ?self.selected,
                    "tick → polling backend"
                );
                self.poll()
            }
            Message::AnimTick => {
                let step = ANIM_TICK_MS as f32 / 1000.0 / ANIM_CYCLE_SECS;
                self.anim_phase = (self.anim_phase + step) % 1.0;
                Task::none()
            }
            Message::WindowFocused(focused) => {
                if focused != self.window_focused {
                    tracing::debug!(focused, "window focus changed");
                }
                self.window_focused = focused;
                if focused {
                    self.poll()
                } else {
                    Task::none()
                }
            }
            Message::SelectSession(id) => {
                self.selected = Some(id.clone());
                self.refresh_peers(id)
            }
            Message::SessionsLoaded(Ok(sessions)) => {
                self.daemon_reachable = true;
                if self.daemon_started_at.is_none() {
                    self.daemon_started_at = Some(chrono::Utc::now());
                }
                if sessions == self.sessions {
                    return Task::none();
                }
                self.diff_and_record(&sessions);
                self.sessions = sessions;
                if self.selected.is_none() {
                    self.selected = self.sessions.first().map(|s| s.id.clone());
                }
                if let Some(id) = self.selected.clone() {
                    self.refresh_peers(id)
                } else {
                    Task::none()
                }
            }
            Message::SessionsLoaded(Err(err)) => {
                self.daemon_reachable = false;
                self.record_error(err)
            }
            Message::PeersLoaded(id, Ok(peers)) => {
                if self.peers.get(&id).map(|p| p == &peers).unwrap_or(false) {
                    return Task::none();
                }
                let short = id[..id.len().min(8)].to_string();
                let mut events: Vec<(EventKind, String)> = Vec::new();
                if let Some(prev) = self.peers.get(&id) {
                    for p in &peers {
                        if !prev.iter().any(|q| q.client_id == p.client_id) {
                            events.push((
                                EventKind::Attach,
                                format!("{} attached to {}", p.client_id, short),
                            ));
                        }
                    }
                    for p in prev {
                        if !peers.iter().any(|q| q.client_id == p.client_id) {
                            events.push((
                                EventKind::Detach,
                                format!("{} detached from {}", p.client_id, short),
                            ));
                        }
                    }
                }

                // Drop any selection that no longer references a live peer.
                if let Some(selected) = self.selected_peers.get(&id)
                    && !peers.iter().any(|p| &p.client_id == selected)
                {
                    self.selected_peers.remove(&id);
                }
                self.peers.insert(id, peers);

                let mut follow = Task::none();
                for (kind, msg) in events {
                    follow = self.event_tape_push(kind, msg);
                }
                follow
            }
            Message::PeersLoaded(_, Err(err)) => self.record_error(err),
            Message::OpenSpawnDrawer => {
                self.spawn_drawer.open = true;
                if self.spawn_drawer.port_input.is_empty() {
                    self.spawn_drawer.port_input = "2222".to_string();
                }
                Task::none()
            }
            Message::CloseSpawnDrawer => {
                self.spawn_drawer.open = false;
                Task::none()
            }
            Message::SpawnPortChanged(port) => {
                self.spawn_drawer.port_input = port;
                Task::none()
            }
            Message::SpawnSubmit => {
                let Ok(port) = self.spawn_drawer.port_input.parse::<u16>() else {
                    return self.record_error(format!(
                        "'{}' is not a valid port",
                        self.spawn_drawer.port_input
                    ));
                };
                let backend = self.backend.clone();
                let shell = self.settings.default_shell.clone();
                Task::perform(
                    async move {
                        backend
                            .start_server(port, &shell)
                            .await
                            .map_err(|e| e.to_string())
                    },
                    |r| act(Message::SpawnDone(r)),
                )
            }
            Message::SpawnDone(Ok(info)) => {
                self.spawn_drawer.open = false;
                self.spawn_drawer.port_input.clear();
                let id = info.id.clone();
                let follow = self.event_tape_push(
                    EventKind::Spawn,
                    format!("spawned session {} on :{}", info.id, info.port),
                );
                self.sessions.push(info);
                self.selected = Some(id.clone());
                Task::batch([follow, self.refresh_peers(id)])
            }
            Message::SpawnDone(Err(err)) => self.record_error(err),
            Message::UpSession(id) => self.dispatch_session_action(SessionAction::Up, id),
            Message::DownSession(id) => self.dispatch_session_action(SessionAction::Down, id),
            Message::AskKill(id) => {
                self.kill_confirm = Some(id);
                Task::none()
            }
            Message::CancelKill => {
                self.kill_confirm = None;
                Task::none()
            }
            Message::ConfirmKill(id) => {
                self.kill_confirm = None;
                self.dispatch_session_action(SessionAction::Kill, id)
            }
            Message::SessionActionDone(action, id, Ok(())) => {
                let (kind, verb) = match action {
                    SessionAction::Up => (EventKind::Wake, "woke"),
                    SessionAction::Down => (EventKind::Sleep, "slept"),
                    SessionAction::Kill => (EventKind::Kill, "killed"),
                };
                let short = id[..id.len().min(8)].to_string();
                let follow = self.event_tape_push(kind, format!("{} session {}", verb, short));
                if matches!(action, SessionAction::Kill) {
                    self.sessions.retain(|s| s.id != id);
                    self.peers.remove(&id);
                    self.peer_sorts.remove(&id);
                    self.selected_peers.remove(&id);
                    if self.selected.as_ref() == Some(&id) {
                        self.selected = self.sessions.first().map(|s| s.id.clone());
                    }
                }
                Task::batch([follow, self.poll()])
            }
            Message::SessionActionDone(_, _, Err(err)) => self.record_error(err),
            Message::SelectPeer(session_id, client_id) => {
                if self.selected_peers.get(&session_id) == Some(&client_id) {
                    self.selected_peers.remove(&session_id);
                } else {
                    self.selected_peers.insert(session_id, client_id);
                }
                Task::none()
            }
            Message::SortPeers(session_id, category) => {
                let next = match self.peer_sorts.get(&session_id) {
                    Some((c, asc)) if *c == category => (category, !*asc),
                    _ => (category, true),
                };
                self.peer_sorts.insert(session_id, next);
                Task::none()
            }
            Message::KickPeer(session_id, client_id) => {
                let backend = self.backend.clone();
                let s = session_id.clone();
                let c = client_id.clone();
                Task::perform(
                    async move { backend.kick(&s, &c).await.map_err(|e| e.to_string()) },
                    move |r| act(Message::KickDone(session_id.clone(), client_id.clone(), r)),
                )
            }
            Message::KickDone(session_id, client_id, Ok(())) => {
                let follow = self.event_tape_push(
                    EventKind::Detach,
                    format!("kicked {} from {}", client_id, session_id),
                );
                Task::batch([follow, self.refresh_peers(session_id)])
            }
            Message::KickDone(_, _, Err(err)) => self.record_error(err),
            Message::RefreshSession(session_id) => {
                let backend = self.backend.clone();
                let id_for_call = session_id.clone();
                Task::perform(
                    async move {
                        backend
                            .refresh(&id_for_call)
                            .await
                            .map_err(|e| e.to_string())
                    },
                    move |r| act(Message::RefreshDone(session_id.clone(), r)),
                )
            }
            Message::RefreshDone(session_id, Ok(())) => {
                let short = session_id[..session_id.len().min(8)].to_string();
                self.event_tape_push(
                    EventKind::Spawn,
                    format!("refreshed TUI for {}", short),
                )
            }
            Message::RefreshDone(_, Err(err)) => self.record_error(err),
            Message::SettingsShellChanged(shell) => {
                self.settings_draft.default_shell = shell;
                Task::none()
            }
            Message::SettingsHostChanged(host) => {
                self.settings_draft.connect_host = host;
                Task::none()
            }
            Message::SettingsDecorationsChanged(show) => {
                self.settings_draft.show_decorations = show;
                Task::none()
            }
            Message::SettingsAlphaChanged(alpha) => {
                self.settings_draft.background_alpha = alpha.clamp(0.0, 1.0);
                Task::none()
            }
            Message::SettingsBlurChanged(on) => {
                self.settings_draft.enable_blur = on;
                Task::none()
            }
            Message::SettingsHamburgerSideChanged(side) => {
                self.settings_draft.hamburger_side = side;
                Task::none()
            }
            Message::SaveSettings => {
                let decorations_changed =
                    self.settings.show_decorations != self.settings_draft.show_decorations;
                let blur_changed = self.settings.enable_blur != self.settings_draft.enable_blur;
                if let Err(err) = self.settings_draft.save() {
                    return self.record_error(format!("could not save settings: {}", err));
                }
                self.settings = self.settings_draft.clone();
                if decorations_changed {
                    self.apply_decorations();
                }
                if blur_changed {
                    self.apply_blur()
                } else {
                    Task::none()
                }
            }
            Message::RevertSettings => {
                self.settings_draft = self.settings.clone();
                Task::none()
            }
            Message::ResetSettings => {
                self.settings_draft = Settings::default();
                Task::none()
            }
            Message::NavigateTo(page) => {
                let target = self
                    .nav
                    .iter()
                    .find(|e| self.nav.data::<Page>(*e).copied() == Some(page));
                if let Some(entity) = target {
                    self.nav.activate(entity);
                }
                self.menu_open = false;
                self.menu_anchor = None;
                Task::none()
            }
            Message::ToggleSidebar => {
                let active = self.core.nav_bar_active();
                self.core_mut().nav_bar_set_toggled(!active);
                self.menu_open = false;
                self.menu_anchor = None;
                Task::none()
            }
            Message::ApplyInitialSettings => {
                self.apply_decorations();
                self.apply_blur()
            }
            Message::SessionHoverEnter(id) => {
                self.hovered_session = Some(id);
                Task::none()
            }
            Message::SessionHoverExit(id) => {
                if self.hovered_session.as_ref() == Some(&id) {
                    self.hovered_session = None;
                }
                Task::none()
            }
            Message::PeerHoverEnter(s, c) => {
                self.hovered_peer = Some((s, c));
                Task::none()
            }
            Message::PeerHoverExit(s, c) => {
                if self.hovered_peer.as_ref() == Some(&(s, c)) {
                    self.hovered_peer = None;
                }
                Task::none()
            }
            Message::OpenMenu => {
                self.menu_open = true;
                self.menu_anchor = CURSOR_POS.lock().ok().and_then(|g| *g);
                Task::none()
            }
            Message::CloseMenu => {
                self.menu_open = false;
                self.menu_anchor = None;
                Task::none()
            }
            Message::DismissError => {
                self.last_error = None;
                Task::none()
            }
            Message::EventTapeScrolled(viewport) => {
                // Within ~24 px of the bottom counts as "at the end" —
                // re-engage follow mode. Strictly checking == 0 misses
                // the case where the user scrolls all the way down but
                // the offset rounds to a non-zero pixel value.
                let bounds = viewport.bounds();
                let content = viewport.content_bounds();
                let offset = viewport.absolute_offset();
                let max_y = (content.height - bounds.height).max(0.0);
                let from_bottom = (max_y - offset.y).max(0.0);
                let next = from_bottom < 24.0;
                if next != self.event_tape_following {
                    self.event_tape_following = next;
                }
                Task::none()
            }
            Message::EventTapeFollow => {
                self.event_tape_following = true;
                self.scroll_tape_to_end()
            }
        }
    }

    fn nav_model(&self) -> Option<&nav_bar::Model> {
        Some(&self.nav)
    }

    fn on_nav_select(&mut self, id: nav_bar::Id) -> Task<Self::Message> {
        self.nav.activate(id);
        Task::none()
    }

    /// We render the sidebar inline inside `view()` rather than going
    /// through cosmic's `nav_bar()` slot. The framework wraps anything
    /// returned from `nav_bar()` in a container with non-zero padding
    /// on the left/right/bottom — and that padding gutter shows the
    /// wallpaper because no widget of ours paints it (we have no
    /// surface-level fill since `style()` returns None). Letting the
    /// rail be part of our view tree keeps the single `body_tint_style`
    /// wrapper covering the whole window edge-to-edge.
    fn nav_bar(&self) -> Option<Element<'_, cosmic::Action<Self::Message>>> {
        None
    }

    /// `Application::style` is intentionally NOT overridden. cosmic's
    /// default returns `Color::TRANSPARENT` for the wgpu surface clear
    /// when the window isn't maximised — the same path cosmic-term
    /// takes — which keeps the per-frame clear free of any visible
    /// pixels. The Catppuccin tint the user sees comes from the
    /// `theme::body_tint_style` container that wraps the body in
    /// `view()`, painted exactly once per frame instead of as a
    /// repeated clear-colour. Overriding `style()` to return an alpha
    /// background_color, even at the same colour, produced a visible
    /// per-update flash because every state change forced iced to
    /// re-clear the surface to that alpha-blended value before
    /// re-drawing the chrome on top.

    fn view(&self) -> Element<'_, Self::Message> {
        let page = self.active_page();
        let body_page: Element<'_, Self::Message> = match page {
            Page::Fleet => crate::pages::fleet::view(self),
            Page::Daemon => crate::pages::daemon::view(self),
            Page::Settings => crate::pages::settings::view(self),
            Page::About => crate::pages::about::view(self),
        };

        // Inline sidebar — replaces the cosmic `nav_bar()` slot so the
        // body_tint container can cover the whole window without a
        // gutter showing wallpaper. The rail is gated on
        // `nav_bar_active()` so Ctrl+B / the hamburger menu still toggle
        // visibility.
        let body_with_rail: Element<'_, Self::Message> = if self.core().nav_bar_active() {
            let ghost: Element<'_, Self::Message> = container(
                crate::components::ghost_art::view::<Self::Message>(96.0, self.anim_phase),
            )
            .padding([12, 0, 8, 0])
            .center_x(Length::Fill)
            .into();

            let mut items = Column::new().spacing(2).padding([4, 8]);
            for p in [Page::Fleet, Page::Daemon, Page::Settings, Page::About] {
                items = items.push(nav_item(p, p == page));
            }

            let rail: Element<'_, Self::Message> = container(
                Column::new()
                    .push(ghost)
                    .push(items)
                    .height(Length::Fill),
            )
            .width(Length::Fixed(160.0))
            .height(Length::Fill)
            .into();

            let edge: Element<'_, Self::Message> = container(text(""))
                .style(theme::sidebar_edge_style)
                .width(Length::Fixed(1.0))
                .height(Length::Fill)
                .into();

            Row::new()
                .push(rail)
                .push(edge)
                .push(container(body_page).width(Length::Fill).height(Length::Fill))
                .height(Length::Fill)
                .width(Length::Fill)
                .into()
        } else {
            container(body_page)
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        };

        let mut inner = Column::new().push(crate::components::daemon_ribbon::view(self));

        if let Some(banner) = self.error_banner() {
            inner = inner.push(banner);
        }

        // The Catppuccin tint is applied as a single container that
        // wraps the daemon ribbon, sidebar, and body together — covers
        // the whole window edge-to-edge so no gutter shows the
        // wallpaper unbidden.
        let main: Element<'_, Self::Message> = container(
            inner
                .push(body_with_rail)
                .spacing(0)
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .style(theme::body_tint_style(self.settings.effective_alpha()))
        .width(Length::Fill)
        .height(Length::Fill)
        .into();

        // Right-click triggers a popover anchored at the cursor.
        //
        // Two constraints worth remembering:
        // (1) Popover's `Position::Point(p)` adds `p` to the popover's
        //     bounds.position(), so `p` must be RELATIVE to the popover
        //     content's top-left, not window-absolute. cosmic-term gets
        //     this for free because its custom widget already reports
        //     cursor in widget-local coords; ours doesn't, so we subtract
        //     a heuristic origin (sidebar width if visible + a small
        //     header gutter).
        // (2) modal(true) absorbs every mouse event, blocking the
        //     click-outside-to-dismiss flow. We rely on on_close instead,
        //     which only fires when modal is false.
        let mut pop = popover(main);
        if self.menu_open {
            let cursor = self.menu_anchor.unwrap_or(Point::new(120.0, 80.0));
            let nav_offset = if self.core.nav_bar_active() {
                184.0
            } else {
                0.0
            };
            let header_offset = if self.settings.show_decorations {
                48.0
            } else {
                0.0
            };
            let relative = Point::new(
                (cursor.x - nav_offset).max(0.0),
                (cursor.y - header_offset).max(0.0),
            );
            pop = pop
                .popup(self.menu_popup())
                .position(cosmic::widget::popover::Position::Point(relative))
                .on_close(Message::CloseMenu);
        }
        pop.into()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        let focus_and_keys = event::listen_with(|ev, _, _| match ev {
            // Track cursor outside app state so 60-Hz mouse-move events
            // don't trigger iced re-renders. The view reads this at
            // popover-anchor time only.
            cosmic::iced::Event::Mouse(mouse::Event::CursorMoved { position }) => {
                if let Ok(mut guard) = CURSOR_POS.lock() {
                    *guard = Some(position);
                }
                None
            }
            cosmic::iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) => {
                Some(Message::OpenMenu)
            }
            cosmic::iced::Event::Window(window::Event::Focused) => {
                Some(Message::WindowFocused(true))
            }
            cosmic::iced::Event::Window(window::Event::Unfocused) => {
                Some(Message::WindowFocused(false))
            }
            cosmic::iced::Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Character(c),
                modifiers,
                ..
            }) if modifiers.control() && c.as_str() == "b" => Some(Message::ToggleSidebar),
            // ESC dismisses the right-click menu. CloseMenu is a no-op
            // when the menu is already closed, so the global key
            // listener can fire it unconditionally.
            cosmic::iced::Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(keyboard::key::Named::Escape),
                ..
            }) => Some(Message::CloseMenu),
            _ => None,
        });

        if !self.window_focused {
            return focus_and_keys;
        }

        let tick = cosmic::iced::time::every(Duration::from_secs(1)).map(|_| Message::Tick);

        // Only run the ghost animation when the ghost is actually
        // on-screen AND the surface is fully opaque. iced exhibits a
        // visible per-redraw flicker against a transparent compositor
        // surface, so when the user has opted into transparency we
        // hold the ghost still and accept a static brand mark in
        // exchange for a calm window — the user explicitly called
        // out the flickering as nausea-inducing.
        let opaque = !self.settings.enable_blur || self.settings.background_alpha >= 0.999;
        let ghost_visible = opaque
            && (self.core.nav_bar_active()
                || matches!(self.active_page(), Page::About)
                || (matches!(self.active_page(), Page::Fleet) && self.sessions.is_empty()));
        if ghost_visible {
            let anim = cosmic::iced::time::every(Duration::from_millis(ANIM_TICK_MS))
                .map(|_| Message::AnimTick);
            Subscription::batch([tick, anim, focus_and_keys])
        } else {
            Subscription::batch([tick, focus_and_keys])
        }
    }
}

/// Sidebar entry — transparent button with a leading dot when active.
/// Hand-rolled because cosmic's `nav_bar` widget hard-codes a solid
/// `primary.base` background container that we can't override; this
/// keeps the rail flat against the wallpaper.
fn nav_item<'a>(page: Page, selected: bool) -> Element<'a, Message> {
    let marker = if selected { "● " } else { "  " };
    let label = format!("{}{}", marker, page.label());
    button::text(label)
        .on_press(Message::NavigateTo(page))
        .width(Length::Fill)
        .into()
}

impl WispAdmin {
    fn active_page(&self) -> Page {
        self.nav
            .data::<Page>(self.nav.active())
            .copied()
            .unwrap_or(Page::Fleet)
    }

    /// Apply the persisted `show_decorations` setting to cosmic's
    /// header_bar visibility. Toggling iced's `window::toggle_decorations`
    /// turns out to control SERVER-side decorations (compositor-drawn
    /// titlebar) — orthogonal to cosmic's own CLIENT-side header which
    /// is always rendered. The visible bug was that toggling SSD on
    /// stacked it on top of cosmic's CSD ("two top bars"). Flipping
    /// `Core::show_headerbar` directly is what actually controls
    /// cosmic's header rendering.
    fn apply_decorations(&mut self) {
        self.core.window.show_headerbar = self.settings.show_decorations;
    }

    /// Request blur from the compositor for the main window. Toggles
    /// the `ext_background_effect_v1` Wayland protocol via iced's
    /// `enable_blur` / `disable_blur` Tasks — without this the surface
    /// just shows the unblurred wallpaper through the alpha. cosmic-comp
    /// is the only compositor in our target environment that supports
    /// the protocol; on others the call is a harmless no-op (the iced
    /// winit backend silently drops `BlurSurface` actions when the
    /// global isn't bound).
    fn apply_blur(&self) -> Task<Message> {
        let Some(id) = self.core.main_window_id() else {
            tracing::warn!("apply_blur: no main_window_id; blur request dropped");
            return Task::none();
        };
        tracing::info!(
            window_id = ?id,
            enabled = self.settings.enable_blur,
            "apply_blur: dispatching ext_background_effect_v1 task"
        );
        if self.settings.enable_blur {
            cosmic::iced::window::enable_blur(id)
        } else {
            cosmic::iced::window::disable_blur(id)
        }
    }

    /// Right-click context menu, mirroring cosmic-term's pattern: a
    /// Column of `menu_button` rows wrapped in a Dialog-styled
    /// container. Anchored at the cursor via `popover.position(Point)`
    /// in `view()`.
    fn menu_popup(&self) -> Element<'_, Message> {
        let item = |label: &'static str, msg: Message| -> Element<'_, Message> {
            menu::menu_button(vec![text(label).into()])
                .on_press(msg)
                .width(Length::Fill)
                .into()
        };
        container(
            Column::new()
                .push(item("Fleet", Message::NavigateTo(Page::Fleet)))
                .push(item("Daemon", Message::NavigateTo(Page::Daemon)))
                .push(item("Settings", Message::NavigateTo(Page::Settings)))
                .push(item("About", Message::NavigateTo(Page::About)))
                .push(item("Toggle sidebar  (Ctrl+B)", Message::ToggleSidebar))
                .push(item("Close menu  (Esc)", Message::CloseMenu))
                .spacing(0)
                .padding(4)
                .width(Length::Fixed(280.0)),
        )
        .class(cosmic::style::Container::Dialog)
        .into()
    }

    fn error_banner(&self) -> Option<Element<'_, Message>> {
        let err = self.last_error.as_ref()?;
        Some(
            container(
                Row::new()
                    .push(text(format!("⚠ {}", err)))
                    .push(button::standard("dismiss").on_press(Message::DismissError))
                    .spacing(12)
                    .padding(12),
            )
            .style(theme::error_banner_style)
            .width(Length::Fill)
            .into(),
        )
    }

    fn poll(&self) -> Task<Message> {
        let backend = self.backend.clone();
        let sessions_task = Task::perform(
            async move { backend.list_servers().await.map_err(|e| e.to_string()) },
            |r| act(Message::SessionsLoaded(r)),
        );

        if let Some(id) = self.selected.clone() {
            Task::batch([sessions_task, self.refresh_peers(id)])
        } else {
            sessions_task
        }
    }

    fn refresh_peers(&self, id: String) -> Task<Message> {
        let backend = self.backend.clone();
        let id_for_call = id.clone();
        Task::perform(
            async move {
                backend
                    .list_peers(&id_for_call)
                    .await
                    .map_err(|e| e.to_string())
            },
            move |r| act(Message::PeersLoaded(id.clone(), r)),
        )
    }

    fn dispatch_session_action(&self, action: SessionAction, id: String) -> Task<Message> {
        let backend = self.backend.clone();
        let id_for_call = id.clone();
        Task::perform(
            async move {
                match action {
                    SessionAction::Up => backend.up(&id_for_call).await,
                    SessionAction::Down => backend.down(&id_for_call).await,
                    SessionAction::Kill => backend.kill(&id_for_call).await,
                }
                .map_err(|e| e.to_string())
            },
            move |r| act(Message::SessionActionDone(action, id.clone(), r)),
        )
    }

    fn diff_and_record(&mut self, next: &[ServerInfo]) {
        let disappeared: Vec<String> = self
            .sessions
            .iter()
            .filter(|s| !next.iter().any(|n| n.id == s.id))
            .map(|s| {
                let short = &s.id[..s.id.len().min(8)];
                format!("session {} disappeared", short)
            })
            .collect();
        // Discard the follow-tape Task — `diff_and_record` is called
        // from `SessionsLoaded(Ok)` which already issues a follow via
        // its subsequent `refresh_peers` -> `PeersLoaded` cycle. One
        // missed scroll here doesn't compound; the next event push
        // catches us back up to the bottom.
        for msg in disappeared {
            let _ = self.event_tape_push(EventKind::Kill, msg);
        }
    }

    fn record_error(&mut self, err: String) -> Task<Message> {
        // Suppress duplicate-error spam: only push to the tape if this is a
        // different error than the one already showing. Dismissing the
        // banner clears `last_error`, so the next failure cycle re-records.
        let follow = if self.last_error.as_ref() != Some(&err) {
            self.event_tape_push(EventKind::Error, err.clone())
        } else {
            Task::none()
        };
        self.last_error = Some(err);
        follow
    }

    pub fn event_tape_push(&mut self, kind: EventKind, message: String) -> Task<Message> {
        self.event_tape.push(EventEntry {
            at: chrono::Utc::now(),
            kind,
            message,
        });
        if self.event_tape.len() > 200 {
            let drop = self.event_tape.len() - 200;
            self.event_tape.drain(0..drop);
        }
        if self.event_tape_following {
            self.scroll_tape_to_end()
        } else {
            Task::none()
        }
    }

    /// Issues a `scroll_to` Task pinned at y = f32::MAX (clamped to
    /// the actual content height by the runtime) so the tape lands at
    /// its newest entry.
    fn scroll_tape_to_end(&self) -> Task<Message> {
        iced_scrollable::scroll_to(
            EVENT_TAPE_SCROLL.clone(),
            iced_scrollable::AbsoluteOffset {
                x: Some(0.0),
                y: Some(f32::MAX),
            },
        )
    }

    pub fn peer_count(&self, session_id: &str) -> usize {
        self.peers.get(session_id).map(|p| p.len()).unwrap_or(0)
    }

}
