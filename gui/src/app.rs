use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use cosmic::app::{Core, Task};
use cosmic::iced::{event, window, Length, Subscription};
use cosmic::widget::{button, container, text, Column, Row};
use cosmic::Element;

use crate::backend::{CliBackend, PeerInfo, ServerInfo, WispBackend};
use crate::components::peers_table::PeerCategory;
use crate::settings::Settings;
use crate::theme;

pub struct WispAdmin {
    core: Core,
    pub backend: Arc<dyn WispBackend>,
    pub page: Page,
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
}

/// Master cycle for the ghost shimmer — chosen to match the longest SMIL
/// period on the source SVG (the teal stop-colour, 11 s) so all
/// sub-animations complete at least one cycle inside it.
/// `ghost_art::view` decomposes this into per-attribute timings.
const ANIM_CYCLE_SECS: f32 = 11.0;
const ANIM_TICK_MS: u64 = 50;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Fleet,
    Daemon,
    Settings,
    About,
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
    SwitchPage(Page),
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
    SaveSettings,
    RevertSettings,
    ResetSettings,

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

    fn init(core: Core, _flags: Self::Flags) -> (Self, Task<Self::Message>) {
        tracing::info!(app_id = Self::APP_ID, "WispAdmin init");
        let backend: Arc<dyn WispBackend> = Arc::new(CliBackend::new());
        let settings = Settings::load();
        tracing::info!(
            shell = %settings.default_shell,
            host = %settings.connect_host,
            "settings loaded"
        );
        let app = WispAdmin {
            core,
            backend: backend.clone(),
            page: Page::Fleet,
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
        };

        let initial_load = Task::perform(
            async move { backend.list_servers().await.map_err(|e| e.to_string()) },
            |result| act(Message::SessionsLoaded(result)),
        );

        (app, initial_load)
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
            Message::SwitchPage(page) => {
                self.page = page;
                Task::none()
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
                self.record_error(err);
                Task::none()
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

                for (kind, msg) in events {
                    self.event_tape_push(kind, msg);
                }
                Task::none()
            }
            Message::PeersLoaded(_, Err(err)) => {
                self.record_error(err);
                Task::none()
            }
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
                    self.record_error(format!(
                        "'{}' is not a valid port",
                        self.spawn_drawer.port_input
                    ));
                    return Task::none();
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
                self.event_tape_push(
                    EventKind::Spawn,
                    format!("spawned session {} on :{}", info.id, info.port),
                );
                self.sessions.push(info);
                self.selected = Some(id.clone());
                self.refresh_peers(id)
            }
            Message::SpawnDone(Err(err)) => {
                self.record_error(err);
                Task::none()
            }
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
                self.event_tape_push(kind, format!("{} session {}", verb, short));
                if matches!(action, SessionAction::Kill) {
                    self.sessions.retain(|s| s.id != id);
                    self.peers.remove(&id);
                    self.peer_sorts.remove(&id);
                    self.selected_peers.remove(&id);
                    if self.selected.as_ref() == Some(&id) {
                        self.selected = self.sessions.first().map(|s| s.id.clone());
                    }
                }
                self.poll()
            }
            Message::SessionActionDone(_, _, Err(err)) => {
                self.record_error(err);
                Task::none()
            }
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
                self.event_tape_push(
                    EventKind::Detach,
                    format!("kicked {} from {}", client_id, session_id),
                );
                self.refresh_peers(session_id)
            }
            Message::KickDone(_, _, Err(err)) => {
                self.record_error(err);
                Task::none()
            }
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
                );
                Task::none()
            }
            Message::RefreshDone(_, Err(err)) => {
                self.record_error(err);
                Task::none()
            }
            Message::SettingsShellChanged(shell) => {
                self.settings_draft.default_shell = shell;
                Task::none()
            }
            Message::SettingsHostChanged(host) => {
                self.settings_draft.connect_host = host;
                Task::none()
            }
            Message::SaveSettings => {
                if let Err(err) = self.settings_draft.save() {
                    self.record_error(format!("could not save settings: {}", err));
                } else {
                    self.settings = self.settings_draft.clone();
                }
                Task::none()
            }
            Message::RevertSettings => {
                self.settings_draft = self.settings.clone();
                Task::none()
            }
            Message::ResetSettings => {
                self.settings_draft = Settings::default();
                Task::none()
            }
            Message::DismissError => {
                self.last_error = None;
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Self::Message> {
        let body: Element<'_, Self::Message> = match self.page {
            Page::Fleet => crate::pages::fleet::view(self),
            Page::Daemon => crate::pages::daemon::view(self),
            Page::Settings => crate::pages::settings::view(self),
            Page::About => crate::pages::about::view(self),
        };

        let mut layout = Column::new()
            .push(crate::components::daemon_ribbon::view(self))
            .push(self.nav_view());

        if let Some(banner) = self.error_banner() {
            layout = layout.push(banner);
        }

        layout
            .push(container(body).width(Length::Fill).height(Length::Fill))
            .spacing(0)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        let focus = event::listen_with(|ev, _, _| match ev {
            cosmic::iced::Event::Window(window::Event::Focused) => {
                Some(Message::WindowFocused(true))
            }
            cosmic::iced::Event::Window(window::Event::Unfocused) => {
                Some(Message::WindowFocused(false))
            }
            _ => None,
        });

        if self.window_focused {
            let tick = cosmic::iced::time::every(Duration::from_secs(1)).map(|_| Message::Tick);
            let anim = cosmic::iced::time::every(Duration::from_millis(ANIM_TICK_MS))
                .map(|_| Message::AnimTick);
            Subscription::batch([tick, anim, focus])
        } else {
            focus
        }
    }
}

impl WispAdmin {
    fn nav_view(&self) -> Element<'_, Message> {
        let tab = |label: &'static str, page: Page| -> Element<'_, Message> {
            if self.page == page {
                button::suggested(label)
                    .on_press(Message::SwitchPage(page))
                    .into()
            } else {
                button::standard(label)
                    .on_press(Message::SwitchPage(page))
                    .into()
            }
        };

        container(
            Row::new()
                .push(tab("Fleet", Page::Fleet))
                .push(tab("Daemon", Page::Daemon))
                .push(tab("Settings", Page::Settings))
                .push(tab("About", Page::About))
                .spacing(8)
                .padding(12),
        )
        .width(Length::Fill)
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
        for msg in disappeared {
            self.event_tape_push(EventKind::Kill, msg);
        }
    }

    fn record_error(&mut self, err: String) {
        // Suppress duplicate-error spam: only push to the tape if this is a
        // different error than the one already showing. Dismissing the
        // banner clears `last_error`, so the next failure cycle re-records.
        if self.last_error.as_ref() != Some(&err) {
            self.event_tape_push(EventKind::Error, err.clone());
        }
        self.last_error = Some(err);
    }

    pub fn event_tape_push(&mut self, kind: EventKind, message: String) {
        self.event_tape.push(EventEntry {
            at: chrono::Utc::now(),
            kind,
            message,
        });
        if self.event_tape.len() > 200 {
            let drop = self.event_tape.len() - 200;
            self.event_tape.drain(0..drop);
        }
    }

    pub fn peer_count(&self, session_id: &str) -> usize {
        self.peers.get(session_id).map(|p| p.len()).unwrap_or(0)
    }

}
