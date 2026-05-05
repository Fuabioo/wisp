// CliBackend — phase-1 backend that shells out to `wisp <cmd> --json`. See
// docs/adr/0002-cosmic-admin-gui.md for the phasing rationale.

use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use serde::Deserialize;
use tokio::process::Command;

use super::{PeerInfo, ServerInfo, SessionStatus, WispBackend};

pub struct CliBackend {
    binary: String,
    socket_override: Option<PathBuf>,
}

#[derive(Debug, Deserialize)]
struct ActionResult {
    #[serde(default)]
    ok: bool,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    port: Option<u16>,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TailEnvelope {
    #[serde(default)]
    tail: String,
}

impl CliBackend {
    pub fn new() -> Self {
        Self {
            binary: std::env::var("WISP_BIN").unwrap_or_else(|_| "wisp".to_string()),
            socket_override: std::env::var("WISP_SOCKET").ok().map(PathBuf::from),
        }
    }

    fn build_command(&self, args: &[&str]) -> Command {
        let mut cmd = Command::new(&self.binary);
        cmd.arg("--json");
        if let Some(sock) = &self.socket_override {
            cmd.arg("--socket").arg(sock);
        }
        for arg in args {
            cmd.arg(arg);
        }
        cmd
    }

    async fn exec(&self, args: &[&str]) -> Result<String> {
        tracing::debug!(binary = %self.binary, ?args, "exec");
        let output = self.build_command(args).output().await.map_err(|err| {
            tracing::error!(
                binary = %self.binary, ?args, error = %err,
                "failed to spawn wisp CLI — is the binary on PATH? \
                 Set WISP_BIN to override (e.g. wisp-dev)."
            );
            anyhow!(err).context(format!("spawning `{} {:?}`", self.binary, args))
        })?;

        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

        if output.status.success() {
            return Ok(stdout);
        }

        // Tiered error extraction: envelope (the intended path), then stderr
        // (cobra/fang pretty-printing on pre-dial errors), then stdout (last
        // resort if the binary printed something we don't recognize).
        if let Ok(envelope) = serde_json::from_str::<ActionResult>(stdout.trim())
            && let Some(msg) = envelope.error
        {
            tracing::warn!(?args, %msg, "wisp returned error envelope");
            return Err(anyhow!(msg));
        }

        let msg = if stderr.is_empty() {
            stdout
        } else {
            stderr
        };
        tracing::warn!(?args, status = %output.status, %msg, "wisp exited non-zero");
        Err(anyhow!("wisp exited with {}: {}", output.status, msg))
    }

    async fn exec_json<T: serde::de::DeserializeOwned>(&self, args: &[&str]) -> Result<T> {
        let raw = self.exec(args).await?;
        let trimmed = raw.trim();
        serde_json::from_str(trimmed).with_context(|| {
            format!(
                "decoding JSON from `wisp {:?}`; got: {}",
                args,
                truncate_for_log(trimmed)
            )
        })
    }

    async fn exec_action(&self, args: &[&str]) -> Result<ActionResult> {
        let envelope = self.exec_json::<ActionResult>(args).await?;
        if !envelope.ok {
            return Err(anyhow!(envelope.error.unwrap_or_else(|| {
                "wisp action returned ok=false with no error message".to_string()
            })));
        }
        Ok(envelope)
    }

    async fn action_by_id(&self, verb: &str, session_id: &str) -> Result<()> {
        self.exec_action(&[verb, session_id]).await.map(|_| ())
    }
}

fn truncate_for_log(s: &str) -> String {
    if s.len() <= 240 {
        return s.to_string();
    }
    let cut = (0..=240).rev().find(|i| s.is_char_boundary(*i)).unwrap_or(0);
    format!("{}…", &s[..cut])
}

impl Default for CliBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl WispBackend for CliBackend {
    async fn list_servers(&self) -> Result<Vec<ServerInfo>> {
        self.exec_json(&["ps"]).await
    }

    async fn list_peers(&self, session_id: &str) -> Result<Vec<PeerInfo>> {
        self.exec_json(&["peers", session_id]).await
    }

    async fn start_server(&self, port: u16) -> Result<ServerInfo> {
        let port_str = port.to_string();
        let envelope = self.exec_action(&["server", "--port", &port_str]).await?;
        Ok(ServerInfo {
            id: envelope
                .id
                .ok_or_else(|| anyhow!("`wisp server --json` returned no id"))?,
            port: envelope.port.unwrap_or(port),
            status: SessionStatus::Active,
        })
    }

    async fn up(&self, session_id: &str) -> Result<()> {
        self.action_by_id("up", session_id).await
    }

    async fn down(&self, session_id: &str) -> Result<()> {
        self.action_by_id("down", session_id).await
    }

    async fn kill(&self, session_id: &str) -> Result<()> {
        self.action_by_id("kill", session_id).await
    }

    async fn kick(&self, session_id: &str, client_id: &str) -> Result<()> {
        self.exec_action(&["kick", session_id, client_id])
            .await
            .map(|_| ())
    }

    async fn refresh(&self, session_id: &str) -> Result<()> {
        self.action_by_id("refresh", session_id).await
    }

    async fn get_tail(&self, session_id: &str) -> Result<String> {
        let envelope = self.exec_json::<TailEnvelope>(&["tail", session_id]).await?;
        Ok(envelope.tail)
    }
}
