// Cosmic-native table for the peers list. Defines the column enum
// (`PeerCategory`) and row payload (`PeerItem`) wired through libcosmic's
// `widget::table` Model + ItemInterface contract. The actual table widget
// is constructed at the call site (pages::fleet) so closures that produce
// app messages can stay close to the message enum.

use std::borrow::Cow;
use std::cmp::Ordering;
use std::fmt;

use chrono::{DateTime, Utc};
use cosmic::iced::Length;
use cosmic::widget::Icon;
use cosmic::widget::table::model::category::{ItemCategory, ItemInterface};

use crate::backend::PeerInfo;
use crate::components::util;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PeerCategory {
    #[default]
    Client,
    Window,
    Remote,
    Attached,
}

impl fmt::Display for PeerCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            PeerCategory::Client => "Client",
            PeerCategory::Window => "Window",
            PeerCategory::Remote => "Remote",
            PeerCategory::Attached => "Attached",
        };
        f.write_str(label)
    }
}

impl ItemCategory for PeerCategory {
    fn width(&self) -> Length {
        match self {
            PeerCategory::Client | PeerCategory::Remote => Length::FillPortion(2),
            PeerCategory::Window | PeerCategory::Attached => Length::FillPortion(1),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PeerItem {
    pub client_id: String,
    pub width: u32,
    pub height: u32,
    pub remote_addr: String,
    pub connected_at: DateTime<Utc>,
}

impl PeerItem {
    pub fn from_info(info: &PeerInfo) -> Self {
        Self {
            client_id: info.client_id.clone(),
            width: info.width,
            height: info.height,
            remote_addr: info.remote_addr.clone(),
            connected_at: info.connected_at,
        }
    }
}

impl ItemInterface<PeerCategory> for PeerItem {
    fn get_icon(&self, _category: PeerCategory) -> Option<Icon> {
        None
    }

    fn get_text(&self, category: PeerCategory) -> Cow<'static, str> {
        match category {
            PeerCategory::Client => Cow::Owned(self.client_id.clone()),
            PeerCategory::Window => Cow::Owned(format!("{}×{}", self.width, self.height)),
            PeerCategory::Remote => Cow::Owned(self.remote_addr.clone()),
            PeerCategory::Attached => {
                let elapsed = Utc::now().signed_duration_since(self.connected_at);
                Cow::Owned(util::humanize_duration(elapsed))
            }
        }
    }

    fn compare(&self, other: &Self, category: PeerCategory) -> Ordering {
        match category {
            PeerCategory::Client => self.client_id.cmp(&other.client_id),
            PeerCategory::Window => {
                (self.width, self.height).cmp(&(other.width, other.height))
            }
            PeerCategory::Remote => self.remote_addr.cmp(&other.remote_addr),
            PeerCategory::Attached => self.connected_at.cmp(&other.connected_at),
        }
    }
}

pub const COLUMN_ORDER: [PeerCategory; 4] = [
    PeerCategory::Client,
    PeerCategory::Window,
    PeerCategory::Remote,
    PeerCategory::Attached,
];
