<p align="center">
  <svg width="200" height="200" viewBox="0 0 200 200" xmlns="http://www.w3.org/2000/svg">
    <!-- Wisp SVG representation -->
    <path d="M100 20C60 20 40 60 40 100C40 140 60 180 100 180C140 180 160 140 160 100C160 60 140 20 100 20Z" fill="#9966FF" opacity="0.8"/>
    <path d="M100 40C75 40 60 70 60 100C60 130 75 160 100 160C125 160 140 130 140 100C140 70 125 40 100 40Z" fill="#CC99FF" opacity="0.9"/>
    <!-- Simple ghost/wisp face -->
    <circle cx="85" cy="90" r="8" fill="#333"/>
    <circle cx="115" cy="90" r="8" fill="#333"/>
    <path d="M 90 120 Q 100 130 110 120" stroke="#333" stroke-width="4" fill="transparent" stroke-linecap="round"/>
  </svg>
  <h1 align="center">Wisp 👻</h1>
  <p align="center">A paranormal daemon for spawning and sharing magic terminal sessions anywhere.</p>
</p>

Wisp is a lightweight, background daemon that allows you to spawn "terminal emulator agnostic" PTY sessions and share them across multiple clients. Start a session on your PC, join it via an SSH tunnel, hop in from an iPad using Blink/Termius, and type together in real-time. It's like having an invisible entity sharing your keyboard!

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
