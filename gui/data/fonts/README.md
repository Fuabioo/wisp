# Fonts

The design plan specifies three faces — Departure Mono (display), Geist Sans
(body), JetBrains Mono (technical). v1 ships with whatever cosmic-text picks
as the system default; bundling the brand fonts is a polish step.

To bundle, drop the `.otf` / `.ttf` files into this directory and load them
from `gui/src/main.rs` via `cosmic::iced::font::load(include_bytes!(...))`
before `cosmic::app::run`.

Sources (free for redistribution, check current licenses before bundling):
- Departure Mono — https://departuremono.com
- Geist Sans — https://vercel.com/font
- JetBrains Mono — https://www.jetbrains.com/lp/mono/
