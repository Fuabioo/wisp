#!/usr/bin/env bash
# Wisp end-to-end harness. Drives the full flow with tmux + ssh, asserts on
# captured pane output, exits non-zero on first failure. Builds wisp first.
#
# Override defaults via env: PORT, KEEP_ARTIFACTS=1 (skip cleanup for debugging).

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
WISP_BIN="$ROOT/wisp"
PORT="${PORT:-22222}"
SESS="wisp-e2e-$$"
WORKDIR="$(mktemp -d -t wisp-e2e.XXXXXX)"
SOCK="$WORKDIR/wisp.sock"
DAEMON_LOG="$WORKDIR/daemon.log"
SESS_ID=""
DAEMON_PID=""

log()  { printf '\n--- %s\n' "$*"; }
fail() {
    printf '\n!!! %s\n' "$*" >&2
    if [[ -n "${SESS_ID:-}" ]]; then
        printf '\n--- last alice pane ---\n' >&2
        tmux capture-pane -t "$SESS:alice" -p 2>/dev/null >&2 || true
    fi
    if [[ -f "$DAEMON_LOG" ]]; then
        printf '\n--- daemon log (tail) ---\n' >&2
        tail -n 30 "$DAEMON_LOG" >&2 || true
    fi
    exit 1
}

cleanup() {
    set +e
    if [[ "${KEEP_ARTIFACTS:-0}" == "1" ]]; then
        printf '\n[KEEP_ARTIFACTS] workdir=%s tmux=%s\n' "$WORKDIR" "$SESS" >&2
        return
    fi
    [[ -n "${SESS_ID:-}" ]] && "$WISP_BIN" --socket "$SOCK" kill "$SESS_ID" >/dev/null 2>&1
    tmux kill-session -t "$SESS" 2>/dev/null
    [[ -n "${DAEMON_PID:-}" ]] && kill "$DAEMON_PID" 2>/dev/null
    sleep 0.1
    [[ -n "${DAEMON_PID:-}" ]] && kill -9 "$DAEMON_PID" 2>/dev/null
    rm -rf "$WORKDIR"
}
trap cleanup EXIT

retry() {
    local attempts=$1; shift
    local i
    for ((i=0; i<attempts; i++)); do
        if "$@"; then return 0; fi
        sleep 0.2
    done
    return 1
}

tcp_open() { (echo > "/dev/tcp/127.0.0.1/$1") 2>/dev/null; }

WISP=( "$WISP_BIN" --socket "$SOCK" )

peers_contain() {
    # fang routes styled output to stderr; merge so grep sees it
    "${WISP[@]}" peers "$SESS_ID" 2>&1 | grep -q "$1"
}
peers_missing() {
    ! peers_contain "$1"
}

SSH_FLAGS="-p $PORT -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o LogLevel=ERROR"
ssh_cmd() { printf 'ssh %s %s@localhost' "$SSH_FLAGS" "$1"; }

require() {
    command -v "$1" >/dev/null 2>&1 || fail "missing required command: $1"
}

require tmux
require ssh
require go

# ---------------------------------------------------------------------------
log "build wisp"
( cd "$ROOT" && go build -o wisp . )

mkdir -p "$WORKDIR/.ssh"

log "start daemon (workdir=$WORKDIR)"
( cd "$WORKDIR" && WISP_SOCKET="$SOCK" "$WISP_BIN" daemon ) >"$DAEMON_LOG" 2>&1 &
DAEMON_PID=$!
retry 50 test -S "$SOCK" || fail "daemon socket missing at $SOCK"

log "start ssh server :$PORT"
"${WISP[@]}" server --port "$PORT"
retry 50 tcp_open "$PORT" || fail "ssh server not listening on :$PORT"

SESS_ID=$("${WISP[@]}" ps 2>&1 | grep -oE '[a-f0-9]{8}' | head -n1)
[[ -n "$SESS_ID" ]] || fail "could not parse session id from wisp ps"
log "session id = $SESS_ID"

log "ssh alice + bob via tmux"
tmux new-session  -d -s "$SESS" -n alice -x 200 -y 50 "$(ssh_cmd alice)"
tmux new-window      -t "$SESS" -n bob                "$(ssh_cmd bob)"
# Keep windows revivable so kicked clients can be respawned without losing the
# tmux session itself.
tmux set-option -w -t "$SESS:alice" remain-on-exit on
tmux set-option -w -t "$SESS:bob"   remain-on-exit on

log "wait for both peers"
retry 50 peers_contain alice-1 || fail "alice-1 never joined"
retry 50 peers_contain bob-1   || fail "bob-1 never joined"

# CHECK 1 - wisp peers shows both
log "[1/5] wisp peers shows both"
peers_contain alice-1 || fail "alice-1 missing from wisp peers"
peers_contain bob-1   || fail "bob-1 missing from wisp peers"

# CHECK 2 - !> opens menu with List peers
log "[2/5] !> opens pause menu with 'List peers'"
tmux send-keys -t "$SESS:alice" '!>'
sleep 0.5
tmux capture-pane -t "$SESS:alice" -p | grep -q "List peers" \
    || fail "pause menu missing 'List peers' entry"

log "    navigate to peer list, assert peers shown"
tmux send-keys -t "$SESS:alice" j
tmux send-keys -t "$SESS:alice" Enter
sleep 0.4
PANE=$(tmux capture-pane -t "$SESS:alice" -p)
echo "$PANE" | grep -q alice-1 || fail "peer list missing alice-1"
echo "$PANE" | grep -q bob-1   || fail "peer list missing bob-1"

log "    return to shell"
tmux send-keys -t "$SESS:alice" Escape
sleep 0.2
tmux send-keys -t "$SESS:alice" Escape
sleep 0.3

# CHECK 3 - kick removes the client
log "[3/5] wisp kick removes bob"
"${WISP[@]}" kick "$SESS_ID" bob-1
retry 50 peers_missing bob-1 || fail "bob-1 still listed after kick"

# CHECK 4 - alice reconnects as alice-1, not alice-2
log "[4/5] alice reconnect re-uses suffix 1 (userCounts decrement)"
# Disconnect alice via the kick RPC — sidesteps bubbletea/shell input timing.
"${WISP[@]}" kick "$SESS_ID" alice-1 >/dev/null 2>&1
retry 50 peers_missing alice-1 || fail "alice-1 still listed after kick"
sleep 0.3
tmux respawn-window -k -t "$SESS:alice" "$(ssh_cmd alice)"
retry 50 peers_contain alice-1 || fail "alice didn't rejoin as alice-1"
peers_contain alice-2 && fail "alice was assigned alice-2 (userCounts didn't decrement)" || true

# CHECK 5 - existing RPC surface still works
log "[5/5] existing CLI: ps / down / up / kill still work"
"${WISP[@]}" ps 2>&1 | grep -q "$SESS_ID" || fail "ps did not list session"
"${WISP[@]}" down "$SESS_ID" >/dev/null 2>&1
sleep 0.2
"${WISP[@]}" up "$SESS_ID" >/dev/null 2>&1
sleep 0.2
"${WISP[@]}" kill "$SESS_ID" >/dev/null 2>&1
SESS_ID=""  # already gone, suppress trap-cleanup warning
sleep 0.2
"${WISP[@]}" ps 2>&1 | grep -qE '[a-f0-9]{8}' && fail "session still listed after kill" || true

log "✅ all checks passed"
