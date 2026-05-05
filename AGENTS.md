# Wisp - Agent Guidelines

This document provides context, architectural details, and conventions to help AI agents work effectively in the Wisp codebase.

## Overview
Wisp is a lightweight daemon and CLI written in Go for spawning and sharing magic terminal (PTY) sessions. It uses a daemon-client architecture where a central daemon manages multiple SSH servers, each wrapping a shared PTY session multiplexed to multiple users.

## Project Structure
Note: The Go module path is `github.com/Fuabioo/wisp`.

- `cmd/`: CLI commands built with `spf13/cobra` (root, daemon, server, up, down, kill, ps).
- `internal/core/`: The core business logic.
  - `daemon.go`: Daemon implementation (RPC server) and session management.
  - `pty.go`: PTY spawning and multiplexing logic (`PTYManager`).
  - `menu.go`: Interactive TUI menu (triggered via `!>`).
  - `ghost.go`: Contains embedded ASCII art.
- `docs/`: Architectural Decision Records (ADRs) and specifications.

## Architecture & Data Flow
1. **Daemon:** A long-running process (`just daemon`) listening on a Unix socket (default `$XDG_RUNTIME_DIR/wisp.sock`; override via `--socket` or `WISP_SOCKET`). Exposes RPC methods (e.g., `Daemon.StartServer`, `Daemon.KillServer`, `Daemon.ListPeers`, `Daemon.KickPeer`).
2. **CLI Clients:** Commands like `wisp server` or `wisp down` send RPC calls to the daemon socket instead of doing the work themselves.
3. **SSH Server:** Each session runs an independent SSH server (via `charm.land/wish/v2` and `github.com/charmbracelet/ssh`) on a specific port.
4. **PTY Multiplexing:** `PTYManager` spawns a single shell (`$SHELL` or `zsh`). A goroutine (`broadcast()`) reads from the PTY and pushes bytes to all connected SSH clients. It listens to window size changes and dynamically resizes the PTY to the minimum dimensions of all active clients.
5. **Intercepting Input:** The `HandleSession` loop intercepts client input. If it detects the `!>` sequence, it pauses input forwarding and brings up a local interactive menu using `bubbletea/v2` via `RunMenu`.

## Concurrency Patterns & Gotchas
- **Mutexes:** Both `Daemon` and `PTYManager` rely heavily on `sync.Mutex`. Always lock when mutating `servers` maps or iterating through active `socks` (connections).
- **Deadlocks:** Avoid blocking RPC calls. Be careful when invoking cleanup functions (e.g., `onClose` in `PTYManager`) that they do not recursively lock the `Daemon` mutex.
- **Client Disconnection:** If a write to a client socket fails in the broadcast loop, the client is dropped silently by deleting it from the `socks` map. Ensure proper cleanup in `defer` statements.
- **TUI Escaping:** The TUI pause menu requires capturing the specific byte sequence `!>`. To send a literal `!`, there is an intentional delay (250ms) to check if `>` follows.

## Development Commands
The project uses `just` as a command runner (see `justfile`):
- `just build` - Builds an optimized Go binary with `trimpath` and `ldflags`.
- `just daemon` - Runs the wisp daemon.
- `just run` - Runs the project normally.
- `just fmt` - Formats Go code.
- `just tidy` - Tidies go modules.
- `just build-micro` - Creates an ultra-compressed build using `upx`.
- `just install-deps` - Installs binary analysis tools (`goda`, `gsa`) and `upx`.
- `just e2e` - Runs `scripts/e2e.sh`, the tmux-driven end-to-end harness (builds wisp, spawns isolated daemon, drives ssh clients, asserts on captured output).

## Dependencies & Ecosystem
- Relies heavily on the Charm ecosystem (`charm.land/*/v2`): `wish/v2`, `ssh` (v1), `lipgloss/v2`, `bubbletea/v2`, `fang/v2`. When building TUI components or styling, refer to Charm standard practices.
- CLI uses `spf13/cobra`.
- PTY management handled by `github.com/creack/pty`.
