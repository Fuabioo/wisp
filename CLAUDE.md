# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

Wisp is a Go daemon for spawning and sharing PTY-backed terminal sessions over SSH. The repository directory is named `prism` but the Go module and built binary are both `wisp`.

## Commands

Build/run is driven by `just` (see `justfile`):

- `just build` — optimized build with version vars injected via `-ldflags` (`wisp/cmd.Version`, `CommitSHA`, `BuildDate`). Output: `./wisp`.
- `just build-micro` — `build` + `upx --best --lzma` compression.
- `just daemon` — `go run . daemon` (starts the daemon).
- `just run` — `go run .`.
- `just fmt` / `just tidy` — `go fmt ./...` / `go mod tidy`.
- `just install-dev` — installs micro build to `~/.local/bin/wisp-dev`.
- `just provision` — installs to `~/.local/bin/wisp` and registers a user-level systemd unit (`~/.config/systemd/user/wisp.service`) running `wisp daemon`.

There are no tests yet; `go test ./...` is a no-op.

CLI surface (cobra + charmbracelet/fang):

- `wisp daemon` — long-running process; owns all sessions.
- `wisp server --port N` — asks the daemon to spawn a new SSH server.
- `wisp ps` / `wisp up <id>` / `wisp down <id>` / `wisp kill <id>` — lifecycle by 8-char UUID prefix.

## Architecture

Two-process model communicating over a Unix socket at `/tmp/wisp.sock` via Go's `net/rpc` (gob).

- **Daemon process** (`cmd/daemon.go` → `internal/core/daemon.go`):
  - Registers `*core.Daemon` for RPC, listens on `/tmp/wisp.sock`.
  - Holds `servers map[string]*ServerSession` guarded by a single `sync.Mutex`. Each session bundles `ServerInfo`, a `*ssh.Server` (charmbracelet/wish), and a `*PTYManager`.
  - RPC methods (`StartServer`, `ListServers`, `KillServer`, `DownServer`, `UpServer`) all follow `func(req, res) error` with pointer args — required by `net/rpc`.
- **CLI subcommands** (`cmd/*.go`): each `RunE` dials the Unix socket, calls a single RPC, prints styled output. Keep this thin — business logic stays in `internal/core`.

Per-session SSH server is built with `wish.NewServer` using a per-port host key file at `.ssh/term_info_<port>_ed25519` (relative to daemon CWD; the daemon must run in a directory where this is writable).

**`PTYManager` (`internal/core/pty.go`) is the heart of the system.**
- One PTY per session, started by spawning `$SHELL` (fallback `zsh`) under `creack/pty`. Multiple SSH clients are *fanned out* from the same `*os.File`: a single `broadcast()` goroutine reads from the PTY and writes to every connected `ssh.Session` in `socks`.
- Each connected client runs `HandleSession`, which reads bytes from the SSH session and forwards them to the PTY. A `!>` digraph (must arrive within 250ms) is intercepted and opens the bubbletea-based pause menu (`menu.go`) instead of being forwarded; a lone `!` followed by anything else (or a timeout) is replayed verbatim.
- `Resize` tracks each client's window dimensions and calls `pty.Setsize` with the per-axis minimum across all clients — this is the "minimum viable dimension" feature so no single small client breaks the others.
- When the underlying PTY EOFs, `broadcast` closes all sockets and invokes `onClose` (set by the daemon to remove the session from its map and close the SSH server).

`StartServer` and `UpServer` both call `createSshServer`; `DownServer` only closes the SSH server but keeps the `PTYManager` alive so `UpServer` can re-attach. `KillServer` tears down both.

`internal/core/ghost.go` embeds `pet.txt` as `GhostArt` (used in MOTD and `--version` output via `cmd/root.go`'s `fang.WithVersion`).

## Conventions specific to this repo

- Module path is `wisp` (no domain). Imports look like `wisp/cmd`, `wisp/internal/core`.
- Version metadata lives in `cmd/root.go` as package-level vars overridden by `-ldflags -X 'wisp/cmd.Version=...'` etc. — keep the names in sync if you rename.
- The Unix socket path `/tmp/wisp.sock` is hardcoded in both `cmd/daemon.go` and every client subcommand. If you change it, update both sides.
- All daemon RPC handlers must keep the `func(req *T, res *U) error` shape or `net/rpc` will silently skip them.
- Style helpers (`successStyle`, `accentStyle`) are in `cmd/root.go` — reuse them rather than re-creating `lipgloss` styles per command.

## Docs

- `docs/spec/wisp-spec.md` — original product spec.
- `docs/adr/0001-daemon-client-architecture.md` — rationale for the daemon/CLI split. Read before proposing structural changes (Chesterton's Fence).
