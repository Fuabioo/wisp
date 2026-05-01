# ADR 0001: Daemon-Client Architecture

## Status
Accepted

## Context
Initially, the Prism prototype functioned as a standalone binary where executing `go run . server --port 8082` spawned both the PTY and the SSH server in the foreground. If a user wanted multiple independent shared terminal environments, they would have to run multiple foreground processes. Additionally, managing these instances (checking statuses, killing them, or pausing them) required interacting directly with the OS process manager (e.g., finding PIDs and sending SIGTERMs).

## Decision
We decided to adopt a Daemon-Client architecture, heavily inspired by tools like Docker or Vagrant.

1. **Daemon (`prism daemon`)**: A long-running background process that holds the state of all terminal sessions. It listens on a Unix socket (`/tmp/prism.sock`) and exposes an RPC API.
2. **CLI Client (`prism <command>`)**: A lightweight client that connects to the daemon to issue instructions.
3. **UUID Tracking**: Instead of just relying on ports, the daemon generates a unique 8-character UUID for every requested server session, allowing for precise lifecycle management.

## Consequences

**Positive:**
- Centralized state management: `prism ps` can cleanly list all running, active, and down servers across the entire machine.
- Decoupled lifecycles: A user can bring a server `down` (closing the SSH listener to reject new connections) while the background PTY shell continues to run safely within the daemon's memory, to be brought `up` later.
- Clean CLI UX: The user experience matches modern developer tooling expectations.

**Negative:**
- Increased complexity: Requires managing RPC calls, Unix sockets, and concurrent state locks within the daemon to prevent race conditions.
- Single point of failure: If the daemon crashes, all active PTY sessions and SSH servers are immediately lost.