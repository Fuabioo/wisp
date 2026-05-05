use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

mod cli;
mod jsonrpc;

pub use cli::CliBackend;
#[allow(unused_imports)]
pub use jsonrpc::JsonRpcBackend;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SessionStatus {
    Active,
    Down,
    #[serde(other)]
    Unknown,
}

impl SessionStatus {
    pub fn label(self) -> &'static str {
        match self {
            SessionStatus::Active => "Active",
            SessionStatus::Down => "Asleep",
            SessionStatus::Unknown => "Unknown",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServerInfo {
    #[serde(rename = "ID")]
    pub id: String,
    #[serde(rename = "Port")]
    pub port: u16,
    #[serde(rename = "Status")]
    pub status: SessionStatus,
}

impl ServerInfo {
    pub fn is_active(&self) -> bool {
        matches!(self.status, SessionStatus::Active)
    }

    pub fn short_id(&self) -> &str {
        &self.id[..self.id.len().min(8)]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PeerInfo {
    #[serde(rename = "ClientID")]
    pub client_id: String,
    #[serde(rename = "Width")]
    pub width: u32,
    #[serde(rename = "Height")]
    pub height: u32,
    #[serde(rename = "RemoteAddr")]
    pub remote_addr: String,
    #[serde(rename = "ConnectedAt")]
    pub connected_at: DateTime<Utc>,
}

#[async_trait]
pub trait WispBackend: Send + Sync {
    async fn list_servers(&self) -> anyhow::Result<Vec<ServerInfo>>;
    async fn list_peers(&self, session_id: &str) -> anyhow::Result<Vec<PeerInfo>>;
    async fn start_server(&self, port: u16) -> anyhow::Result<ServerInfo>;
    async fn up(&self, session_id: &str) -> anyhow::Result<()>;
    async fn down(&self, session_id: &str) -> anyhow::Result<()>;
    async fn kill(&self, session_id: &str) -> anyhow::Result<()>;
    async fn kick(&self, session_id: &str, client_id: &str) -> anyhow::Result<()>;
}
