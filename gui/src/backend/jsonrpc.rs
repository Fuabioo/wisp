// JsonRpcBackend — phase-2 stub. See docs/adr/0002-cosmic-admin-gui.md.
// Will speak JSON-over-Unix directly to the daemon once TODO.md item
// "Optional gRPC or JSON-over-Unix-socket transport" is in place.

#![allow(dead_code)]

use anyhow::{bail, Result};
use async_trait::async_trait;

use super::{PeerInfo, ServerInfo, WispBackend};

pub struct JsonRpcBackend {
    socket: std::path::PathBuf,
}

impl JsonRpcBackend {
    pub fn new(socket: impl Into<std::path::PathBuf>) -> Self {
        Self {
            socket: socket.into(),
        }
    }
}

#[async_trait]
impl WispBackend for JsonRpcBackend {
    async fn list_servers(&self) -> Result<Vec<ServerInfo>> {
        bail!("JsonRpcBackend not implemented (phase 2 — see ADR 0002): {}", self.socket.display())
    }
    async fn list_peers(&self, _session_id: &str) -> Result<Vec<PeerInfo>> {
        bail!("JsonRpcBackend not implemented (phase 2 — see ADR 0002)")
    }
    async fn start_server(&self, _port: u16) -> Result<ServerInfo> {
        bail!("JsonRpcBackend not implemented (phase 2 — see ADR 0002)")
    }
    async fn up(&self, _session_id: &str) -> Result<()> {
        bail!("JsonRpcBackend not implemented (phase 2 — see ADR 0002)")
    }
    async fn down(&self, _session_id: &str) -> Result<()> {
        bail!("JsonRpcBackend not implemented (phase 2 — see ADR 0002)")
    }
    async fn kill(&self, _session_id: &str) -> Result<()> {
        bail!("JsonRpcBackend not implemented (phase 2 — see ADR 0002)")
    }
    async fn kick(&self, _session_id: &str, _client_id: &str) -> Result<()> {
        bail!("JsonRpcBackend not implemented (phase 2 — see ADR 0002)")
    }
    async fn refresh(&self, _session_id: &str) -> Result<()> {
        bail!("JsonRpcBackend not implemented (phase 2 — see ADR 0002)")
    }
    async fn get_tail(&self, _session_id: &str) -> Result<String> {
        bail!("JsonRpcBackend not implemented (phase 2 — see ADR 0002)")
    }
}
