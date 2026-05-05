// Column definitions and sort helper for the peers table. The actual
// rendering happens in `pages::fleet::peers_view`, which builds a custom
// hover-aware table out of `cosmic::widget::list::button` rows + a
// sortable header. We don't use `cosmic::widget::table` because it
// renders rows with a static container style (no hover hook) and panics
// on default context-menu builders.

use std::cmp::Ordering;
use std::fmt;

use crate::backend::PeerInfo;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PeerCategory {
    Client,
    Window,
    Remote,
    Attached,
}

impl fmt::Display for PeerCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

impl PeerCategory {
    pub fn label(self) -> &'static str {
        match self {
            PeerCategory::Client => "CLIENT",
            PeerCategory::Window => "WINDOW",
            PeerCategory::Remote => "REMOTE",
            PeerCategory::Attached => "ATTACHED",
        }
    }

    /// Relative width portion (sums to 6 across the four columns).
    pub fn width_portion(self) -> u16 {
        match self {
            PeerCategory::Client | PeerCategory::Remote => 2,
            PeerCategory::Window | PeerCategory::Attached => 1,
        }
    }
}

pub fn compare(a: &PeerInfo, b: &PeerInfo, category: PeerCategory) -> Ordering {
    match category {
        PeerCategory::Client => a.client_id.cmp(&b.client_id),
        PeerCategory::Window => (a.width, a.height).cmp(&(b.width, b.height)),
        PeerCategory::Remote => a.remote_addr.cmp(&b.remote_addr),
        PeerCategory::Attached => a.connected_at.cmp(&b.connected_at),
    }
}
