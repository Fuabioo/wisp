<table>
<tr>
<td valign="middle">

Wisp 👻 is a lightweight, background daemon that allows you to spawn "terminal emulator agnostic" PTY sessions and share them across multiple clients. Start a session on your PC, join it via an SSH tunnel, hop in from an iPad using Blink/Termius, and type together in real-time. It's like having an invisible entity sharing your keyboard!

</td>
<td align="center" valign="middle" width="240">
<img src="assets/logo-wisp.svg" alt="Wisp logo" width="220"/>
</td>
</tr>
</table>

## Features

- **Shared PTY Sessions**: Multiple clients connect to the exact same underlying shell (defaulting to your `$SHELL` or `zsh`).
- **Daemon Architecture**: Spin up one daemon, manage infinite terminal sessions across different ports seamlessly.
- **Dynamic Resizing**: Automatically calculates the "minimum viable dimension" across all active clients so your UI never breaks.
- **TUI Pause Menu**: Type `!>` quickly to pause your client interaction and selectively disconnect, leaving everyone else perfectly undisturbed.
- **Lifecycle Management**: Spin sessions `up`, bring them `down`, or `kill` them completely via UUID.

## Installation

```bash
git clone https://github.com/Fuabioo/wisp
cd wisp
go build -o wisp
```

## Usage

1. Start the daemon (usually in a background task or screen/tmux):
   ```bash
   ./wisp daemon
   ```

2. Start a new shared server session:
   ```bash
   ./wisp server --port 8082
   ```

3. See active sessions:
   ```bash
   ./wisp ps
   ```

4. Connect to a session (locally or remotely via your IP/Cloudflare Tunnel):
   ```bash
   ssh -p 8082 localhost
   ```

5. Manage session state:
   ```bash
   ./wisp down <uuid>
   ./wisp up <uuid>
   ./wisp kill <uuid>
   ```

## Menu
While in an active shared session, type `!>` rapidly to open the Pause Menu!
