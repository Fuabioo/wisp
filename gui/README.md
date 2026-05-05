# wisp-admin

Pop!\_OS COSMIC-native admin GUI for the [wisp](..) daemon. Built with
[`libcosmic`](https://github.com/pop-os/libcosmic) (Rust + iced).

The design lives in `~/.claude/plans/help-me-design-a-curried-porcupine.md`
and the architecture rationale in `../docs/adr/0002-cosmic-admin-gui.md`.

## Status

Phase 1 (v1) — admin-only fleet console. Talks to the daemon by shelling
out to `wisp <cmd> --json`. Polling at 1 Hz.

## System dependencies

`libcosmic` needs Wayland, xkbcommon, fontconfig, and libudev development
headers. On Pop!\_OS / Debian:

```
sudo apt install libwayland-dev libxkbcommon-dev libfontconfig1-dev libudev-dev
```

Confirm with:

```
pkg-config --exists wayland-client libxkbcommon fontconfig libudev && echo ok
```

## Build & run

The crate is a standalone Rust project under `gui/`. Use the `just` recipes
from the wisp repo root:

```
just gui-check    # type-check only, no binary
just gui-build    # debug build
just gui-run      # debug build + run
just gui-release  # release build
```

The GUI looks for the `wisp` binary on `$PATH`. Override with `WISP_BIN`,
and override the daemon socket with `WISP_SOCKET` (matches the CLI).

## Layout

```
src/
├── main.rs                # entry; runs cosmic::app::run::<WispAdmin>
├── app.rs                 # cosmic::Application impl, message loop, polling
├── theme.rs               # Phosphor Ghost palette (anchored to cmd/root.go)
├── subscriptions.rs       # placeholder for phase-3 streaming
├── backend/
│   ├── mod.rs             # WispBackend trait + ServerInfo/PeerInfo types
│   ├── cli.rs             # phase-1: shell-out to `wisp --json`
│   └── jsonrpc.rs         # phase-2 stub
├── pages/
│   ├── fleet.rs           # default view (rail + spine + tape)
│   ├── daemon.rs          # daemon health
│   └── about.rs           # version + tribute
└── components/
    ├── daemon_ribbon.rs   # always-visible top row
    ├── session_orb.rs     # rail row
    ├── peer_row.rs        # peer list row in the spine
    ├── event_tape.rs      # bottom log strip
    └── ghost_art.rs       # pet.txt → text widget (single source of truth)

data/
├── desktop/               # .desktop file for installation
├── fonts/                 # drop bundled fonts here (see README inside)
└── icons/                 # rasterized brand assets
```

## Pinning libcosmic

`Cargo.toml` pulls libcosmic from `branch = "master"`. After the first
`cargo build` succeeds, run `cargo update -p libcosmic` and commit the
resulting `Cargo.lock` to pin a specific SHA. ADR 0002 describes the bump
procedure.
