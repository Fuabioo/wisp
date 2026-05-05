package core

import (
	"fmt"
	"io"
	"strings"
	"time"

	tea "charm.land/bubbletea/v2"
	"github.com/charmbracelet/ssh"
)

type viewState int

const (
	viewMenu viewState = iota
	viewPeerList
)

type menuAction int

const (
	actionNone menuAction = iota
	actionResume
	actionDisconnect
	actionShowPeers
)

type menuItem struct {
	label  string
	action menuAction
}

type MenuModel struct {
	session  ssh.Session
	clientID string
	peersFn  func() []PeerInfo

	state    viewState
	items    []menuItem
	cursor   int
	choice   string
	peers    []PeerInfo
	peersErr string
}

// RunMenu runs the pause menu over an SSH session. peersFn is called lazily
// each time the user opens the peer list — pass a snapshot function (like
// PTYManager.Peers) so the menu always shows current state. width/height are
// the client's PTY window dimensions; bubbletea v2 needs them explicitly when
// the output isn't a *os.File TTY.
func RunMenu(s ssh.Session, input io.Reader, clientID string, width, height int, peersFn func() []PeerInfo) (string, error) {
	m := MenuModel{
		session:  s,
		clientID: clientID,
		peersFn:  peersFn,
		items: []menuItem{
			{label: "Resume", action: actionResume},
			{label: "List peers", action: actionShowPeers},
			{label: "Disconnect", action: actionDisconnect},
		},
	}
	p := tea.NewProgram(
		m,
		tea.WithInput(input),
		tea.WithOutput(s),
		tea.WithWindowSize(width, height),
	)
	finalModel, err := p.Run()
	if err != nil {
		return "", err
	}
	if finalM, ok := finalModel.(MenuModel); ok {
		return finalM.choice, nil
	}
	return "", nil
}

func (m MenuModel) Init() tea.Cmd {
	return nil
}

func (m MenuModel) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	switch m.state {
	case viewPeerList:
		return m.updatePeerList(msg)
	default:
		return m.updateMenu(msg)
	}
}

func (m MenuModel) updateMenu(msg tea.Msg) (tea.Model, tea.Cmd) {
	key, ok := msg.(tea.KeyMsg)
	if !ok {
		return m, nil
	}
	switch key.String() {
	case "ctrl+c", "q", "esc":
		m.choice = "Resume"
		return m, tea.Quit
	case "up", "k":
		if m.cursor > 0 {
			m.cursor--
		}
	case "down", "j":
		if m.cursor < len(m.items)-1 {
			m.cursor++
		}
	case "enter", " ":
		return m.activate()
	}
	return m, nil
}

func (m MenuModel) activate() (tea.Model, tea.Cmd) {
	item := m.items[m.cursor]
	switch item.action {
	case actionResume:
		m.choice = "Resume"
		return m, tea.Quit
	case actionDisconnect:
		m.choice = "Disconnect"
		return m, tea.Quit
	case actionShowPeers:
		m.peers = nil
		m.peersErr = ""
		if m.peersFn != nil {
			m.peers = m.peersFn()
		} else {
			m.peersErr = "peer list unavailable"
		}
		m.state = viewPeerList
		return m, nil
	}
	return m, nil
}

func (m MenuModel) updatePeerList(msg tea.Msg) (tea.Model, tea.Cmd) {
	key, ok := msg.(tea.KeyMsg)
	if !ok {
		return m, nil
	}
	switch key.String() {
	case "esc", "q", "enter", " ", "backspace":
		m.state = viewMenu
		m.peers = nil
		m.peersErr = ""
	case "ctrl+c":
		m.choice = "Resume"
		return m, tea.Quit
	}
	return m, nil
}

func (m MenuModel) View() tea.View {
	var body string
	switch m.state {
	case viewPeerList:
		body = m.renderPeerList()
	default:
		body = m.renderMenu()
	}
	v := tea.NewView(body)
	v.AltScreen = true
	return v
}

func (m MenuModel) renderMenu() string {
	var b strings.Builder
	fmt.Fprintf(&b, "\n%s\n\n🌈 Wisp Pause Menu 🌈\n👤 Client: %s\n\n", GhostArt, m.clientID)
	for i, item := range m.items {
		cursor := " "
		if m.cursor == i {
			cursor = ">"
		}
		fmt.Fprintf(&b, "%s %s\n", cursor, item.label)
	}
	b.WriteString("\nPress j/k to move, enter to select, esc to return to session.\n")
	return b.String()
}

func (m MenuModel) renderPeerList() string {
	var b strings.Builder
	fmt.Fprintf(&b, "\n🌈 Peers in this session 🌈\n👤 You: %s\n\n", m.clientID)
	if m.peersErr != "" {
		fmt.Fprintf(&b, "⚠ %s\n", m.peersErr)
	} else if len(m.peers) == 0 {
		b.WriteString("(no peers attached)\n")
	} else {
		now := time.Now()
		for _, p := range m.peers {
			marker := " "
			if p.ClientID == m.clientID {
				marker = "*"
			}
			fmt.Fprintf(&b, "%s %-20s %dx%d  %s  %s\n",
				marker,
				p.ClientID,
				p.Width, p.Height,
				p.RemoteAddr,
				humanizeDuration(now.Sub(p.ConnectedAt)),
			)
		}
		b.WriteString("\n* = you\n")
	}
	b.WriteString("\nPress esc/q to return to the menu.\n")
	return b.String()
}

func humanizeDuration(d time.Duration) string {
	if d < time.Minute {
		return fmt.Sprintf("%ds", int(d.Seconds()))
	}
	if d < time.Hour {
		return fmt.Sprintf("%dm", int(d.Minutes()))
	}
	return fmt.Sprintf("%dh%dm", int(d.Hours()), int(d.Minutes())%60)
}
