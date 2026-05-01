# Prism Specification

## Vision
Prism is a daemonized tool that allows a user to spawn "terminal emulator agnostic" PTY sessions and share them across multiple clients. The primary goal is to seamlessly spawn a terminal session on a host machine and securely attach/detach to it from anywhere (local network, remote via SSH tunnels, iPad via Blink/Termius, etc.) without losing state. It also serves as an excellent foundation for testing AI agents acting within shared tmux/shell sessions.

## Core Features
1. **Shared PTY Sessions**: Multiple clients connecting to the same Prism server port share the exact same underlying shell (defaulting to `$SHELL` or `zsh`).
2. **Daemon Architecture**: A single background daemon manages multiple independent PTY sessions on different ports, similar to Docker or Vagrant.
3. **Session Lifecycles**: Sessions are identified by an 8-character UUID. They can be created, temporarily deactivated (`down`), brought back online (`up`), or permanently destroyed (`kill`).
4. **Dynamic Resizing**: When multiple clients with different terminal window sizes connect, Prism automatically calculates the "minimum viable dimension" across all active clients to prevent rendering issues and line wrapping bugs.
5. **Client Interception TUI (Pause Menu)**: By typing `!>` quickly, a client triggers a local, full-screen Bubbletea TUI pause menu without sending those keystrokes to the shared PTY. This allows clients to selectively disconnect from the session without impacting other users.
6. **Client Identifiers**: Clients are tracked per-username and assigned incrementing IDs (e.g., `fuabioo-1`, `fuabioo-2`).

## Architecture Details
- **Daemon**: Runs in the background and listens on a Unix domain socket (`/tmp/prism.sock`). Manages a map of active/down `ServerSession` structs.
- **CLI Client**: A Cobra-based Go binary that communicates with the daemon via RPC. Commands include `server`, `ps`, `up`, `down`, and `kill`.
- **Networking**: Each shared session spins up its own Charm `wish` SSH server on a dynamically requested port. Clients connect via standard SSH protocols.
- **Event Handling**: 
  - Standard input is read via a custom buffered byte-channel to allow sequence interception (like the `!>` chord or arrow-key escape sequences).
  - PTY `input/output errors` triggers a clean disconnection of all clients, signaling the daemon to clean up the session.