# ADR 0002: COSMIC-Native Admin GUI (`wisp-admin`)

## Status
Accepted

## Context
The wisp daemon was always envisioned as a primitive with three consumers
(see `MEMORY.md`): the in-session Bubbletea pause menu, the Cobra CLI, and a
dedicated **management UI**. The first two are shipped. The UI is the open
slot.

Without a GUI, an operator who runs more than one or two sessions has to hop
between `wisp ps`, `wisp peers <id>`, `wisp up/down/kill <id>`, and `wisp
kick <id> <client-id>` to maintain situational awareness. That is the
shape of an admin interface — it just lives across seven commands today.

The user runs Pop!\_OS with the new (Rust-based) COSMIC desktop. They want a
*native* application — one that inherits the system theme, accent color,
light/dark transitions, panel integration, and notifications.

## Decision
We will build `wisp-admin`, a Pop!\_OS COSMIC-native desktop application,
using **`libcosmic`** (System76's Rust framework, which wraps `iced`). The
crate lives at `gui/` inside this repo (not as a Cargo workspace member at
the repo root, to keep the Go tooling untouched).

### Rejected alternatives
- **Web app (Electron / Tauri / browser tab).** Wisp's daemon socket is
  per-user (`$XDG_RUNTIME_DIR`) and the audience is a single operator; a
  browser tab is an impostor of native chrome on a system the user has
  shaped to their taste. Rejected.
- **GTK + libadwaita.** Would integrate with GNOME, not COSMIC. The user
  runs COSMIC; GTK on COSMIC works but does not match the system's accent,
  theming, or panel idioms. Rejected.
- **Bubbletea TUI as the admin surface.** The TUI landing-screen TODO
  (TODO.md "Interactive TUI") is a *separate* consumer for keyboard-only
  / SSH'd-in operators. It is complementary, not competitive. Kept on the
  TODO list.

### Backend transport — phased

The daemon currently speaks `net/rpc` with `gob` encoding. A Rust client
cannot decode gob without a custom codec, and TODO.md already flags
"Optional gRPC or JSON-over-Unix-socket transport alongside `net/rpc` for
non-Go clients" as the right path. The GUI is the forcing function for that
work, but we will not gate the GUI on it.

Three phases:

| Phase | Transport | Status |
|-------|-----------|--------|
| **1** | GUI shells out to `wisp <cmd> --json` via `tokio::process` | Implemented: `--json` flag on every CLI command (cmd/json.go). |
| **2** | GUI talks JSON-over-Unix directly to the daemon (alongside the existing `net/rpc` listener) | Pending; tracked in TODO.md. |
| **3** | Streaming events (SSE/WebSocket-over-Unix) for live attach/detach/sleep/kill | Pending; tracked in TODO.md "Streaming RPCs". |

The Rust `WispBackend` trait (`gui/src/backend/mod.rs`) abstracts all three
phases so the UI layer is unchanged when we swap implementations.

### Brand DNA contract

The CLI's lipgloss palette in `cmd/root.go` is the **canonical brand
palette**. The GUI's `theme.rs` mirrors those values explicitly:

| lipgloss color | GUI token | Hex |
|----------------|-----------|-----|
| `99` (purple)  | `accent.wisp` | `#9B6EFF` |
| `212` (pink)   | `accent.brand` | `#FF87D7` |
| `204` (rose)   | `accent.rose` | `#FF8FAF` |

The GUI introduces two additional brand tokens — `accent.alive` (phosphor
green `#7CE3A9`, used for "Active" pills and the daemon heartbeat) and
`danger` (`#FF6B7A`, kill button only). Any future change to the CLI
palette must be reflected in `gui/src/theme.rs`; the contract is kept
intentional via a comment block in that file pointing back to this ADR.

The pixel ghost (`internal/core/pet.txt`) is the second canonical asset.
The GUI's app icon, empty states, and splash all derive from it — single
source of truth so brand stays anchored.

## Consequences

**Positive**
- Native COSMIC integration: system accent, light/dark, notifications,
  panel positioning come for free.
- Forces the daemon to grow a JSON transport (TODO.md item) — strictly
  better than the current Go-only `net/rpc + gob` lock-in.
- `--json` on every CLI command is independently useful: scripting, future
  shell completions, and any third-party consumer all benefit, not just
  this GUI.
- Three-consumer architecture is realized: pause menu, CLI, GUI all hit
  the same `Daemon.*` RPC surface.

**Negative**
- Adds a Rust toolchain to the project's build matrix. Existing Go-only
  contributors are unaffected (the `gui/` crate is independent), but
  releasing a binary that includes the GUI requires both toolchains.
- `libcosmic` is a moving target (System76 iterates fast). Pin to a
  specific git SHA in `gui/Cargo.toml` and document the bump procedure
  in `gui/README.md`.
- v1 polls instead of streaming. Benign at 1 Hz, but the daemon's TODO
  item for streaming RPCs becomes a latent dependency for full-fidelity
  UX (live event tape, instant peer flash on attach).
