# TODO

Features and improvements not yet implemented. Not a roadmap — just ideas worth picking up.

## Interactive TUI

When `wisp` is invoked with no subcommand, drop into a bubbletea TUI exposing every CLI feature (server start/stop, `ps`, `up`/`down`/`kill`, attach) instead of just printing help. Subcommands keep working for scripting; the bare invocation becomes the discoverable entry point.

- [ ] Session list view backed by `ListServers` RPC, with live refresh
- [ ] Inline actions for `up` / `down` / `kill` and `server` (port prompt)
- [ ] Surface MOTD / ghost art on the landing screen

## Configuration file

A user config file (e.g. `~/.config/wisp/config.toml`) read by the daemon at startup so Wisp is customizable without rebuilding.

- [ ] Default shell (currently `$SHELL` → `zsh` fallback, hardcoded in `pty.go`)
- [ ] Pause-menu trigger sequence and timeout window (currently `!>` with 250ms, hardcoded in `pty.go`)
- [ ] Daemon socket path (currently `/tmp/wisp.sock` — would also be a security win to default to `$XDG_RUNTIME_DIR/wisp.sock`)
- [ ] Default port for `wisp server` (currently `2222`)
- [ ] SSH host key location (currently `.ssh/term_info_<port>_ed25519` relative to daemon CWD)
- [ ] PTY / broadcast buffer sizes

## Event hooks

User-defined shell commands triggered by daemon events, similar to Claude Code's hooks. Hook config in the same config file; event payload passed via environment variables or JSON on stdin. Use cases: desktop notifications, log lines, `ntfy.sh` pushes, audio bell, custom audit trails.

- [ ] `on_client_connect` (event payload: session id, port, client id, remote addr)
- [ ] `on_client_disconnect`
- [ ] `on_session_start` / `on_session_end`
- [ ] `on_pause_menu_open` / `on_pause_menu_close`
- [ ] Per-hook async/sync flag (most hooks should be fire-and-forget)

## Pause menu

- [ ] Configurable trigger sequence (currently hardcoded `!>`)
- [ ] **List peers** — show every client currently in the session
- [ ] **Send message** — broadcast a one-line note to all peers via a small overlay
- [ ] **Lock session** — refuse new SSH connections until unlocked
- [ ] **Kick peer** (owner only)
- [ ] **Toggle read-only** — observe-only mode for the current client

## Sessions

- [ ] **Named sessions** — `wisp server --name foo`, so `up`/`down`/`kill` don't require remembering UUIDs
- [ ] **Persistence across daemon restart** — keep session metadata and port reservations; PTYs can't survive a daemon restart but reconnects should work
- [ ] **Recording** — append PTY output to a file for asciinema-style replay (`wisp record <id>` / `--record-on-start`)
- [ ] **Read-only viewer mode** as a per-client startup flag

## Security

- [ ] **Authentication** — currently anyone who can reach the SSH port joins as that user. Options: per-session password, public-key allowlist, one-time tokens.
- [ ] **Owner vs. guest permissions** for menu actions (kick, lock, configure)
- [ ] Move daemon socket to `$XDG_RUNTIME_DIR` (per-user) instead of `/tmp` (world-readable)
- [ ] Optional connection rate limiting

## Observability

- [ ] Structured logs (`log/slog`) instead of stdlib `log`
- [ ] `wisp logs <id>` — tail logs for a specific session via the daemon RPC

## Code quality

- [ ] **Tests** — currently zero. PTY fan-out, the `!>` digraph state machine, the daemon RPC handlers, and `updateSizeLocked` minimum-dimension calculation are all worth covering.
- [ ] **Code coverage badge** — wire up `go test -cover` in CI (e.g. GitHub Actions + Codecov or coveralls) and surface the badge in `README.md`.
- [ ] Replace `chanReader` byte-by-byte channel pipeline with a batched `bufio.Reader`-based fan-in. Same correctness, fewer allocations, batched PTY writes.
- [ ] `userCounts` is a lifetime counter that never decrements — clients get suffixes like `fabio-47` after enough churn. Should track currently-connected count.
- [ ] Graceful daemon shutdown — current SIGTERM handler closes immediately; should send a goodbye banner and let in-flight writes drain.
