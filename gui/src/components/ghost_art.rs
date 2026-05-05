// Pixel ghost — the brand leitmotif. Single source of truth is
// `internal/core/pet.txt` (see ADR 0002). We `include_str!` it directly so
// the GUI binary stays in sync with the daemon's MOTD ghost.

use cosmic::widget::{container, text, Column};
use cosmic::Element;

pub const GHOST_ART: &str = include_str!("../../../internal/core/pet.txt");

pub fn view<'a, Message: 'a>() -> Element<'a, Message> {
    container(
        Column::new()
            .push(text(GHOST_ART).font(cosmic::font::mono()).size(14))
            .spacing(2),
    )
    .padding(12)
    .into()
}
