// User-editable settings persisted to disk. Loaded once at startup, saved
// when the Settings page's "Save" button fires.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct Settings {
    /// Shell binary to run when spawning a new session. Empty string =
    /// hand off to the daemon's `$SHELL` / zsh fallback. The system
    /// default is captured at first launch via the SHELL env var.
    pub default_shell: String,

    /// Hostname (or IP) to display in the SSH connect string. Defaults
    /// to the value of `hostname(1)` so other PCs on the LAN can copy
    /// the string without first guessing where to connect.
    pub connect_host: String,

    /// Whether to render the OS / cosmic-shell window decorations
    /// (header bar with title, close/min/max buttons, nav-bar toggle).
    /// Hide this if you want a leaner chrome — pair with the keyboard
    /// shortcut and right-click menu since the nav-bar toggle goes with
    /// the decorations.
    pub show_decorations: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            default_shell: detect_default_shell(),
            connect_host: detect_hostname(),
            show_decorations: true,
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        let path = config_path();
        match std::fs::read_to_string(&path) {
            Ok(s) => match toml::from_str(&s) {
                Ok(parsed) => parsed,
                Err(err) => {
                    tracing::warn!(?path, %err, "failed to parse settings; using defaults");
                    Self::default()
                }
            },
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Self::default(),
            Err(err) => {
                tracing::warn!(?path, %err, "failed to read settings; using defaults");
                Self::default()
            }
        }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let path = config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let body = toml::to_string_pretty(self)?;
        std::fs::write(&path, body)?;
        tracing::info!(?path, "settings saved");
        Ok(())
    }
}

fn config_path() -> PathBuf {
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
        PathBuf::from(xdg).join("wisp-admin/settings.toml")
    } else if let Some(home) = std::env::var_os("HOME") {
        PathBuf::from(home).join(".config/wisp-admin/settings.toml")
    } else {
        PathBuf::from("settings.toml")
    }
}

/// Reads the host's `hostname` so the connect string defaults to something
/// reachable from another PC. Falls back to "localhost" only if reading
/// fails (rare on Linux).
fn detect_hostname() -> String {
    std::process::Command::new("hostname")
        .output()
        .ok()
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "localhost".to_string())
}

/// Captures the user's current shell so the spawn-default reflects their
/// actual login shell rather than wisp's hard-coded fallback. Empty if
/// `$SHELL` is unset; in that case the daemon picks zsh.
fn detect_default_shell() -> String {
    std::env::var("SHELL").unwrap_or_default()
}
