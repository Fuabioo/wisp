package core

import (
	"fmt"
	"io"
	"log"
	"os"
	"os/exec"
	"sort"
	"strings"
	"sync"
	"sync/atomic"
	"time"

	"charm.land/lipgloss/v2"
	"github.com/charmbracelet/ssh"
	"github.com/creack/pty"
)

// tailCapacity caps the per-session scrollback ring buffer used by GetTail.
// ~64 KiB ≈ 8 full screens of 80×25, enough for a TUI snapshot + a bit of
// history without bloating daemon memory across many sessions.
const tailCapacity = 64 * 1024

// refreshDelay sits between the +1 and -1 PTY resizes done by RefreshAll;
// gives the foreground process a chance to handle SIGWINCH on the bumped
// size before it springs back to the original.
const refreshDelay = 30 * time.Millisecond

type peerEntry struct {
	ClientID    string
	Window      ssh.Window
	RemoteAddr  string
	ConnectedAt time.Time
}

// PeerInfo is the RPC-safe view of a connected client. Kept flat (no charm
// types) so non-Go consumers can decode it.
type PeerInfo struct {
	ClientID    string
	Width       int
	Height      int
	RemoteAddr  string
	ConnectedAt time.Time
}

// tailBuffer is a tail-only ring buffer; old data falls off the front when
// new bytes push past `capacity`. Concurrent writers/snapshot readers are
// serialised through `mu`.
type tailBuffer struct {
	mu       sync.Mutex
	data     []byte
	capacity int
}

func newTailBuffer(capacity int) *tailBuffer {
	return &tailBuffer{capacity: capacity, data: make([]byte, 0, capacity)}
}

func (tb *tailBuffer) write(p []byte) {
	tb.mu.Lock()
	defer tb.mu.Unlock()
	tb.data = append(tb.data, p...)
	if len(tb.data) > tb.capacity {
		excess := len(tb.data) - tb.capacity
		copy(tb.data, tb.data[excess:])
		tb.data = tb.data[:tb.capacity]
	}
}

func (tb *tailBuffer) snapshot() string {
	tb.mu.Lock()
	defer tb.mu.Unlock()
	return string(tb.data)
}

type PTYManager struct {
	file     *os.File
	cmd      *exec.Cmd
	mu       sync.Mutex
	socks    map[ssh.Session]peerEntry
	paused   map[ssh.Session]bool
	onClose  func()
	lastRows uint16
	lastCols uint16
	tail     *tailBuffer

	bytesOut atomic.Uint64
	bytesIn  atomic.Uint64

	port       int
	statusBar  bool
	statusPos  string
	statusText string
	cfg        Config
}

// SessionStats is a snapshot of per-session bandwidth consumption.
type SessionStats struct {
	BytesIn  uint64
	BytesOut uint64
}

type chanReader struct {
	ch <-chan byte
}

func (c chanReader) Read(p []byte) (n int, err error) {
	if len(p) == 0 {
		return 0, nil
	}
	b, ok := <-c.ch
	if !ok {
		return 0, io.EOF
	}
	p[0] = b
	n = 1
	for {
		select {
		case b, ok := <-c.ch:
			if !ok {
				return n, nil
			}
			p[n] = b
			n++
			if n == len(p) {
				return n, nil
			}
		default:
			return n, nil
		}
	}
}

func NewPTYManager(shell string, shadowDir string, env map[string]string, onClose func(), port int, cfg Config) (*PTYManager, error) {
	if shell == "" {
		shell = os.Getenv("SHELL")
	}
	if shell == "" {
		shell = "zsh"
	}
	c := exec.Command(shell)
	c.Env = os.Environ()
	if shadowDir != "" {
		c.Env = prependPath(c.Env, shadowDir)
	}
	for k, v := range env {
		c.Env = setEnv(c.Env, k, v)
	}
	c.Env = setEnvDefault(c.Env, "TERM", "xterm-256color")
	f, err := pty.Start(c)
	if err != nil {
		return nil, fmt.Errorf("start pty: %w", err)
	}

	pm := &PTYManager{
		file:       f,
		cmd:        c,
		socks:      make(map[ssh.Session]peerEntry),
		paused:     make(map[ssh.Session]bool),
		onClose:    onClose,
		tail:       newTailBuffer(tailCapacity),
		port:       port,
		statusBar:  cfg.StatusBar.Enabled,
		statusPos:  cfg.StatusBar.Position,
		statusText: cfg.StatusBar.Suggestion,
		cfg:        cfg,
	}

	go pm.broadcast()
	return pm, nil
}

func (pm *PTYManager) broadcast() {
	buf := make([]byte, 1024)
	for {
		n, err := pm.file.Read(buf)
		if err != nil {
			if err == io.EOF || strings.Contains(err.Error(), "input/output error") {
				log.Println("PTY closed")
				pm.mu.Lock()
				for s := range pm.socks {
					s.Close()
				}
				pm.mu.Unlock()
				if pm.onClose != nil {
					pm.onClose()
				}
				return
			}
			log.Printf("Failed to read from pty: %v", err)
			return
		}

		// Capture for `wisp tail` / GUI console preview before fan-out so
		// dead-client cleanup on write failure can't race the buffer.
		pm.tail.write(buf[:n])

		pm.mu.Lock()
		sessions := make([]ssh.Session, 0, len(pm.socks))
		for s := range pm.socks {
			if !pm.paused[s] {
				sessions = append(sessions, s)
			}
		}
		pm.mu.Unlock()

		var dead []ssh.Session
		for _, s := range sessions {
			nw, err := s.Write(buf[:n])
			pm.bytesOut.Add(uint64(nw))
			if err != nil {
				dead = append(dead, s)
			}
		}

		pm.paintStatusBars(sessions)

		if len(dead) > 0 {
			pm.mu.Lock()
			for _, s := range dead {
				delete(pm.socks, s)
				delete(pm.paused, s)
			}
			pm.mu.Unlock()
		}
	}
}

// Attach registers a new client session with full identity. Called once when
// the SSH session is established.
func (pm *PTYManager) Attach(s ssh.Session, clientID, remoteAddr string, win ssh.Window) {
	pm.mu.Lock()
	defer pm.mu.Unlock()
	pm.socks[s] = peerEntry{
		ClientID:    clientID,
		Window:      win,
		RemoteAddr:  remoteAddr,
		ConnectedAt: time.Now(),
	}
	pm.updateSizeLocked()
}

// Resize updates the window of an existing client session. No-op if the
// session is unknown (e.g. resize event arrived during teardown).
func (pm *PTYManager) Resize(s ssh.Session, win ssh.Window) {
	pm.mu.Lock()
	defer pm.mu.Unlock()
	if entry, ok := pm.socks[s]; ok {
		entry.Window = win
		pm.socks[s] = entry
	}
	pm.updateSizeLocked()
}

// Peers returns a snapshot of the currently attached clients, ordered by
// connection time (oldest first) so polling consumers see a stable list.
func (pm *PTYManager) Peers() []PeerInfo {
	pm.mu.Lock()
	defer pm.mu.Unlock()
	out := make([]PeerInfo, 0, len(pm.socks))
	for _, entry := range pm.socks {
		out = append(out, PeerInfo{
			ClientID:    entry.ClientID,
			Width:       entry.Window.Width,
			Height:      entry.Window.Height,
			RemoteAddr:  entry.RemoteAddr,
			ConnectedAt: entry.ConnectedAt,
		})
	}
	sort.Slice(out, func(i, j int) bool {
		if !out[i].ConnectedAt.Equal(out[j].ConnectedAt) {
			return out[i].ConnectedAt.Before(out[j].ConnectedAt)
		}
		return out[i].ClientID < out[j].ClientID
	})
	return out
}

// Tail returns a copy of the recent PTY output (capped at tailCapacity
// bytes). Safe to call from any goroutine.
func (pm *PTYManager) Tail() string {
	return pm.tail.snapshot()
}

// Stats returns a snapshot of the current bandwidth counters. Safe to call
// from any goroutine.
func (pm *PTYManager) Stats() SessionStats {
	return SessionStats{
		BytesIn:  pm.bytesIn.Load(),
		BytesOut: pm.bytesOut.Load(),
	}
}

// Pause marks a session as paused so broadcast skips it (e.g. while the
// pause menu is shown). Must be paired with Resume.
func (pm *PTYManager) Pause(s ssh.Session) {
	pm.mu.Lock()
	pm.paused[s] = true
	pm.mu.Unlock()
}

// Resume clears the paused flag so broadcast resumes writing to the session.
func (pm *PTYManager) Resume(s ssh.Session) {
	pm.mu.Lock()
	delete(pm.paused, s)
	pm.mu.Unlock()
}

// RefreshAll perturbs the PTY size by +1 then back to the original to
// generate a SIGWINCH on the foreground process. TUIs like claude-code
// repaint their full UI on resize, so this gives a peer who attached
// mid-session a fresh paint without anyone disconnecting.
func (pm *PTYManager) RefreshAll() {
	pm.mu.Lock()
	rows := pm.lastRows
	cols := pm.lastCols
	pm.mu.Unlock()
	if rows == 0 || cols == 0 {
		return
	}
	_ = pty.Setsize(pm.file, &pty.Winsize{Rows: rows + 1, Cols: cols + 1})
	time.Sleep(refreshDelay)
	_ = pty.Setsize(pm.file, &pty.Winsize{Rows: rows, Cols: cols})
}

// Kick closes the SSH session belonging to the given clientID. Returns true
// if a matching client was found. The actual cleanup of socks happens via the
// usual HandleSession defer.
func (pm *PTYManager) Kick(clientID string) bool {
	pm.mu.Lock()
	var target ssh.Session
	for s, entry := range pm.socks {
		if entry.ClientID == clientID {
			target = s
			break
		}
	}
	pm.mu.Unlock()
	if target == nil {
		return false
	}
	target.Close()
	return true
}

func (pm *PTYManager) HandleSession(s ssh.Session, clientID string) {
	defer func() {
		pm.mu.Lock()
		delete(pm.socks, s)
		delete(pm.paused, s)
		pm.updateSizeLocked()
		pm.mu.Unlock()
	}()

	bytesChan := make(chan byte, 1024)
	go func() {
		buf := make([]byte, 1024)
		for {
			n, err := s.Read(buf)
			if err != nil {
				close(bytesChan)
				return
			}
			for i := range n {
				bytesChan <- buf[i]
			}
		}
	}()

	writeByte := func(b byte) bool {
		nw, err := pm.file.Write([]byte{b})
		pm.bytesIn.Add(uint64(nw))
		if err != nil {
			log.Printf("pty write failed for %s: %v", clientID, err)
			return false
		}
		return true
	}

	var pendingBang bool
	var timeoutChan <-chan time.Time

	for {
		select {
		case c, ok := <-bytesChan:
			if !ok {
				if pendingBang {
					writeByte('!')
				}
				return
			}
			if pendingBang {
				pendingBang = false
				timeoutChan = nil
				if c == '>' {
					pm.mu.Lock()
					win := pm.socks[s].Window
					pm.mu.Unlock()
					pm.Pause(s)
					choice, _ := RunMenu(s, chanReader{ch: bytesChan}, clientID, win.Width, win.Height, pm.Peers, pm.ToggleStatusBar, pm.StatusBarEnabled(), pm.RefreshAll)
					pm.Resume(s)
					pm.RefreshAll()
					if choice == "Disconnect" {
						s.Close()
						return
					}
					continue
				}
				if !writeByte('!') {
					return
				}
			}

			if c == '!' {
				pendingBang = true
				timeoutChan = time.After(250 * time.Millisecond)
				continue
			}
			if !writeByte(c) {
				return
			}

		case <-timeoutChan:
			pendingBang = false
			timeoutChan = nil
			if !writeByte('!') {
				return
			}
		}
	}
}

func (pm *PTYManager) updateSizeLocked() {
	if len(pm.socks) == 0 {
		return
	}
	var minRows, minCols uint16
	first := true
	for _, entry := range pm.socks {
		if entry.Window.Width == 0 || entry.Window.Height == 0 {
			continue
		}
		if first {
			minRows = uint16(entry.Window.Height)
			minCols = uint16(entry.Window.Width)
			first = false
		} else {
			if uint16(entry.Window.Height) < minRows {
				minRows = uint16(entry.Window.Height)
			}
			if uint16(entry.Window.Width) < minCols {
				minCols = uint16(entry.Window.Width)
			}
		}
	}
	if first {
		return
	}
	if pm.statusBar && minRows > 0 {
		minRows--
	}
	if minRows == pm.lastRows && minCols == pm.lastCols {
		return
	}
	pm.lastRows, pm.lastCols = minRows, minCols
	pty.Setsize(pm.file, &pty.Winsize{
		Rows: minRows,
		Cols: minCols,
	})
}

// prependPath inserts dir at the front of PATH in the given env slice.
// If PATH is not present, it creates one with only dir.
func prependPath(env []string, dir string) []string {
	prefix := "PATH="
	for i, e := range env {
		if len(e) > 5 && e[:5] == prefix {
			env[i] = prefix + dir + ":" + e[5:]
			return env
		}
	}
	return append(env, prefix+dir)
}

// setEnv sets or overrides a key=value pair in the env slice.
func setEnv(env []string, key, value string) []string {
	prefix := key + "="
	for i, e := range env {
		if len(e) > len(prefix) && e[:len(prefix)] == prefix {
			env[i] = prefix + value
			return env
		}
	}
	return append(env, prefix+value)
}

// setEnvDefault sets key=value only if the key is absent or has an empty
// value. Used to backfill TERM when the daemon runs under a systemd user unit
// that does not export it — without TERM the spawned shell loses terminfo
// capabilities (no colour, broken backspace, autosuggest residue).
func setEnvDefault(env []string, key, value string) []string {
	prefix := key + "="
	for _, e := range env {
		if len(e) > len(prefix) && e[:len(prefix)] == prefix {
			return env
		}
	}
	return append(env, prefix+value)
}

func (pm *PTYManager) Close() {
	pm.mu.Lock()
	defer pm.mu.Unlock()
	pm.file.Close()
	if pm.cmd != nil && pm.cmd.Process != nil {
		pm.cmd.Process.Kill()
	}
}

// StatusBarEnabled reports whether the status bar is currently on.
func (pm *PTYManager) StatusBarEnabled() bool {
	pm.mu.Lock()
	defer pm.mu.Unlock()
	return pm.statusBar
}

// ToggleStatusBar flips the status bar on/off, recalculates the PTY size,
// and triggers a SIGWINCH so all clients repaint. Returns the new state.
func (pm *PTYManager) ToggleStatusBar() bool {
	pm.mu.Lock()
	pm.statusBar = !pm.statusBar
	pm.lastRows = 0
	pm.lastCols = 0
	pm.updateSizeLocked()
	pm.mu.Unlock()
	pm.RefreshAll()
	return pm.StatusBarEnabled()
}

// Port returns the SSH port this session is listening on.
func (pm *PTYManager) Port() int {
	return pm.port
}

// StatusBarPosition returns the configured position ("top" or "bottom").
func (pm *PTYManager) StatusBarPosition() string {
	return pm.statusPos
}

// paintStatusBars repaints the status bar on every attached client after
// each PTY broadcast cycle, so applications that scroll (e.g. claude-code)
// never overwrite the reserved bottom/top row. Only called from broadcast().
func (pm *PTYManager) paintStatusBars(sessions []ssh.Session) {
	if !pm.statusBar {
		return
	}
	pm.mu.Lock()
	theme := pm.cfg.Theme.Dark
	pos := pm.statusPos
	hintText := pm.statusText
	port := pm.port
	pm.mu.Unlock()

	for _, s := range sessions {
		pm.mu.Lock()
		entry, ok := pm.socks[s]
		pm.mu.Unlock()
		if !ok || entry.Window.Width == 0 || entry.Window.Height == 0 {
			continue
		}
		PaintStatusBar(s, port, hintText, theme, pos, entry.Window.Width, entry.Window.Height)
	}
}

// PaintStatusBar paints an ANSI status line onto the given SSH session.
// Exported so the daemon middleware can show the bar on initial attach
// and resize; the steady-state repaint happens via broadcast → paintStatusBars.
func PaintStatusBar(s ssh.Session, port int, hintText string, theme ThemeVariant, pos string, width, height int) {
	row := height - 1
	if pos == "top" {
		row = 0
	}

	bg := lipgloss.Color(theme.PrimaryBG)
	fg := lipgloss.Color(theme.PrimaryFG)
	hintFg := lipgloss.Color(theme.SuggestionFG)

	statusStyle := lipgloss.NewStyle().
		Background(bg).
		Foreground(fg).
		Padding(0, 1)

	hintStyle := lipgloss.NewStyle().
		Background(bg).
		Foreground(hintFg).
		Padding(0, 1)

	portLabel := statusStyle.Render(fmt.Sprintf("[ :%d ]", port))
	hintLabel := hintStyle.Render(hintText)

	pad := width - lipgloss.Width(hintLabel) - lipgloss.Width(portLabel)
	if pad < 0 {
		pad = 0
	}
	bar := hintLabel + strings.Repeat(" ", pad) + portLabel

	// Save cursor → move to indicator row → clear & paint → restore.
	_, _ = s.Write([]byte(fmt.Sprintf("\033[s\033[%d;1H\033[2K%s\033[u", row+1, bar)))
}
