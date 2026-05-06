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
- [ ] **Replay on attach** — when a client joins mid-session, dump the full asciicast byte stream into the terminal before switching to live mode. Terminal emulators converge to the correct visual state by processing all accumulated ANSI escapes, solving the "join a running TUI and see garbage" problem cleanly. Works with the existing ring buffer (64 KiB clip) or a full `.cast` file.
- [ ] **Per-client recording** alongside the merged session recording, using the input-provenance work above.

## Interactive TUI

When `wisp` is invoked with no subcommand, drop into a bubbletea TUI exposing every CLI feature (server start/stop, `ps`, `up`/`down`/`kill`, attach) instead of just printing help. Subcommands keep working for scripting; the bare invocation becomes the discoverable entry point.

- [ ] Session list view backed by `ListServers` RPC, with live refresh
- [ ] Inline actions for `up` / `down` / `kill` and `server` (port prompt)
- [ ] Surface MOTD / ghost art on the landing screen

## Configuration file

A user config file (e.g. `~/.config/wisp/config.toml`) read by the daemon at startup so Wisp is customizable without rebuilding.

- [ ] Default shell in config file (`$SHELL` → `zsh` fallback in `pty.go`; per-session override exists via `wisp server --shell`, but no global config default yet)
- [ ] Pause-menu trigger sequence and timeout window (currently `!>` with 250ms, hardcoded in `pty.go`)
- [ ] Daemon socket path in config file (already defaults to `$XDG_RUNTIME_DIR/wisp.sock` via `$WISP_SOCKET` / `--socket`, but not yet settable via config file)
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
- [ ] **Status bar** — a subtle 1–2 row overlay at the bottom of each client showing session metadata (name/id, peer count, host, your client id). Helps orient users who shelled out and forgot they're inside a remote wisp session. Toggle on/off from the pause menu; disabled by default. Decouples from the PTY viewport (rows reserved from the SSH window, not written to the PTY).

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
- [x] `wisp logs <id>` — shipped as `wisp tail <id>` (64 KiB ring buffer, `Daemon.GetTail` RPC, `cmd/tail.go`)

## Code quality

- [ ] **Tests** — `scripts/e2e.sh` covers the full pause-menu / peers / kick / userCounts flow via tmux + ssh. Targeted Go unit tests for the `!>` digraph state machine, `updateSizeLocked` minimum-dimension calculation, and the RPC handlers are still pending.
- [ ] **Code coverage badge** — wire up `go test -cover` in CI (e.g. GitHub Actions + Codecov or coveralls) and surface the badge in `README.md`.
- [ ] Replace `chanReader` byte-by-byte channel pipeline with a batched `bufio.Reader`-based fan-in. Same correctness, fewer allocations, batched PTY writes.
- [x] `userCounts` is a lifetime counter that never decrements — clients get suffixes like `fabio-47` after enough churn. Should track currently-connected count.
- [ ] Graceful daemon shutdown — current SIGTERM handler closes immediately; should send a goodbye banner and let in-flight writes drain.

## Recently discovered opportunities

Items surfaced during a codebase audit against the TODO list. Not duplicates of the above — genuine gaps.

- [ ] **Expose daemon version over RPC** — `cmd/root.go` already carries `Version`, `CommitSHA`, and `BuildDate` vars; the COSMIC GUI's daemon page wants them but only gets reachability + uptime today. Add `Daemon.GetVersion` RPC.
- [ ] **CI pipeline** — GitHub Actions workflow running `go build`, `go vet`, and the e2e harness on push. A `just test` recipe would give it a single entry point.
- [ ] **Live PTY preview in the COSMIC GUI** — the fleet page defers a console pane (`gui/src/pages/fleet.rs`). Real-time terminal rendering in the GUI would be the killer feature.
- [ ] **WebSocket transport** — a browser-based watcher can't speak net/rpc over a Unix socket today. A companion proxy or in-process WebSocket endpoint would open the door to web dashboards and zero-install observers.
- [ ] **Session labels** — `wisp server --label "deploy-staging"` for freeform tagging, filterable in `wisp ps` and the GUI. Lighter than named sessions; pairs well with the sessions list.
- [ ] **Export recordings as asciicast v2** — beyond raw byte capture, produce shareable `.cast` files that asciinema.org and `agg` can render.
- [ ] **SSH public-key allowlist per session** — refinement of the Authentication TODO: read from `~/.ssh/authorized_keys` or a per-session temp file so only known keys can attach.
- [ ] **`just` tab completion** — generate shell completions for all 20+ just recipes so new contributors discover them faster.
- [ ] **Rename GUI to `wisp-desktop`** — purely cosmetic. The marketable name is `wisp-desktop` (cf. Docker Desktop / Claude Desktop) but the crate is still `wisp-admin` internally. Touches `gui/Cargo.toml` (crate + bin name), the `[[bin]]` `name`, justfile recipe names (`gui-*` → maybe `desktop-*`), `APP_ID = "dev.fabiomora.WispAdmin"`, the `.desktop` filename under `gui/data/desktop/`, references in ADR 0002 and `gui/README.md`. README already uses the new name; the rest is non-urgent.
- [ ] **wisp-desktop: stop polling dead sessions** — after a session disappears (killed in the GUI *or* externally via the CLI), the 1 Hz poll keeps issuing `list_peers(<dead-id>)` and surfaces "session \<id\> not found" errors that respawn the moment the user dismisses them. Repro confirmed in the logs:
  ```
  WARN wisp_admin::backend::cli: wisp returned error envelope
       args=["peers", "dcc971b8"] msg=session dcc971b8 not found
  ```
  fires every second indefinitely. Root cause is in `Message::SessionsLoaded(Ok)` (`gui/src/app.rs`): we reconcile `self.sessions` to the new list but never re-check `self.selected`, so if the selected id was removed externally it stays pinned to a dead id and `refresh_peers` keeps firing against it. Fix: in `SessionsLoaded(Ok)`, after `self.sessions = sessions`, if `self.selected` is `Some(id)` and `id` is not in `self.sessions`, clear it (or reassign to `self.sessions.first()`) — same logic that already lives in `SessionActionDone(Kill)`. Bonus: have `record_error` collapse "session \<id\> not found" against a per-id last-seen timestamp so even if the underlying issue resurfaces, the banner doesn't keep redrawing.
