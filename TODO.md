# TODO

Features and improvements not yet implemented. Not a roadmap — just ideas worth picking up.

## RPC as a public surface (daemon-as-platform)

Right now the daemon's `net/rpc` interface is a private CLI-to-daemon detail. Promote it to a stable, documented API so other tools can embed wisp as a primitive instead of shelling out to it. Named sessions and persistence are on the critical path — UUIDs are fine for humans, terrible for programmatic callers.

- [ ] Stabilize and document the RPC schema (request/response types, error codes, version field). Treat breaking changes as semver-major.
- [ ] Optional gRPC or JSON-over-Unix-socket transport alongside `net/rpc` for non-Go clients. Gob-only locks out everyone else.
- [ ] Streaming RPCs for `wisp logs <id>` and live event subscriptions (client connect/disconnect, resize, hook fires).
- [ ] Token / capability auth on the socket once it leaves `/tmp` (see Security TODO) — a public API needs a real authz model.
- [ ] Client SDK (`pkg/client`) with typed wrappers, so the CLI becomes the first consumer rather than the only one.

## Real-time multi-agent steering (swarm orchestration)

The unexplored idea: **two or more AI agents attached to the exact same PTY**, coordinating in real time the way human pair-programmers used to. One agent = one task is the norm; one orchestrator + N subagents is common; *N agents sharing one task with live mutual awareness* is rare. Wisp's fan-out PTY makes it almost free as a substrate.

- [ ] **Edit-lock primitive** — a soft mutex broadcast over the session so attached agents/clients can advertise "I'm editing file X" and others back off. Built on top of the planned `send message` overlay.
- [ ] **Role channels** — driver / navigator / observer roles enforced at the menu layer (read-only mode is the foundation).
- [ ] **Agent-friendly event stream** — structured JSON events (keystroke origin, attach/detach, focus changes) over the RPC streaming surface, so attached agents can reason about who did what without parsing the raw PTY.
- [ ] **Sandboxed swarm preset** — opinionated "spawn N agents in a sandboxed wisp session" recipe (firejail / nsjail / container) as a reference deployment.
- [ ] **Provenance on PTY input** — tag each byte stream with its origin client id so recordings/replays can attribute actions per agent. Useful for post-hoc debugging of "who broke the build."

## AI-agent observability

- [ ] **Agent-as-PTY-process recipe** — documented pattern for running Claude Code / Aider / etc. inside a wisp session so humans can `up` to watch and `down` to leave the agent running.
- [ ] **Live intervention UX** — pause-menu action to inject a one-shot prompt or interrupt without taking over the session.
- [ ] **Attendance hooks** — fire an event when *no* human is attached vs. when one joins, so an agent can adapt verbosity / pacing.

## Attended automation / approval gates

- [ ] **`wisp gate <cmd>`** wrapper — runs `cmd` in a wisp session, pauses at sentinel markers in stdout, and notifies via hooks until a human attaches and confirms.
- [ ] CI integration example (GitHub Actions step that suspends on a wisp gate and posts the join URL).

## Classroom / broadcast mode

- [ ] **Read-only attach** as a per-client flag (`ssh -o "SendEnv=WISP_RO=1"` or a session-level setting).
- [ ] **High-fan-out tuning** — current `socks` write loop is fine for a handful of clients; classrooms want dozens. Profile and batch.

## Recording / replay

- [ ] **asciinema-format recording** of each session (`--record` flag, file per session id). Already on the Sessions list — kept here for cross-reference.
- [ ] **Per-client recording** alongside the merged session recording, using the input-provenance work above.

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
- [x] **List peers** — show every client currently in the session
- [ ] **Send message** — broadcast a one-line note to all peers via a small overlay
- [ ] **Lock session** — refuse new SSH connections until unlocked
- [ ] **Kick peer** (owner only) — RPC + `wisp kick` CLI exist; pause-menu entry is gated on owner-vs-guest permissions
- [ ] **Toggle read-only** — observe-only mode for the current client

## Sessions

- [ ] **Named sessions** — `wisp server --name foo`, so `up`/`down`/`kill` don't require remembering UUIDs
- [ ] **Persistence across daemon restart** — keep session metadata and port reservations; PTYs can't survive a daemon restart but reconnects should work
- [ ] **Recording** — append PTY output to a file for asciinema-style replay (`wisp record <id>` / `--record-on-start`)
- [ ] **Read-only viewer mode** as a per-client startup flag

## Security

- [ ] **Authentication** — currently anyone who can reach the SSH port joins as that user. Options: per-session password, public-key allowlist, one-time tokens.
- [ ] **Owner vs. guest permissions** for menu actions (kick, lock, configure) — `Daemon.KickPeer` RPC exists but enforcement is still TODO
- [x] Move daemon socket to `$XDG_RUNTIME_DIR` (per-user) instead of `/tmp` (world-readable) — default sourced from `$XDG_RUNTIME_DIR`, override via `--socket` / `WISP_SOCKET`
- [ ] Optional connection rate limiting

## Observability

- [ ] Structured logs (`log/slog`) instead of stdlib `log`
- [ ] `wisp logs <id>` — tail logs for a specific session via the daemon RPC

## Code quality

- [ ] **Tests** — `scripts/e2e.sh` covers the full pause-menu / peers / kick / userCounts flow via tmux + ssh. Targeted Go unit tests for the `!>` digraph state machine, `updateSizeLocked` minimum-dimension calculation, and the RPC handlers are still pending.
- [ ] **Code coverage badge** — wire up `go test -cover` in CI (e.g. GitHub Actions + Codecov or coveralls) and surface the badge in `README.md`.
- [ ] Replace `chanReader` byte-by-byte channel pipeline with a batched `bufio.Reader`-based fan-in. Same correctness, fewer allocations, batched PTY writes.
- [x] `userCounts` is a lifetime counter that never decrements — clients get suffixes like `fabio-47` after enough churn. Should track currently-connected count.
- [ ] Graceful daemon shutdown — current SIGTERM handler closes immediately; should send a goodbye banner and let in-flight writes drain.
