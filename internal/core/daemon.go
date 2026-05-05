package core

import (
	"fmt"
	"log"
	"sort"
	"sync"

	"charm.land/lipgloss/v2"
	"charm.land/wish/v2"
	"charm.land/wish/v2/logging"
	"github.com/charmbracelet/ssh"
	"github.com/google/uuid"
)

var (
	// userSuffixes tracks currently-in-use integer suffixes per username so
	// disconnects free up the suffix and reconnects don't drift upward.
	userSuffixes   = make(map[string]map[int]struct{})
	userSuffixesMu sync.Mutex
)

// acquireClientID reserves the smallest unused integer suffix for the given
// user and returns the assembled clientID along with a release func that the
// caller must invoke when the session ends.
func acquireClientID(user string) (string, func()) {
	userSuffixesMu.Lock()
	defer userSuffixesMu.Unlock()
	if userSuffixes[user] == nil {
		userSuffixes[user] = make(map[int]struct{})
	}
	n := 1
	for {
		if _, taken := userSuffixes[user][n]; !taken {
			break
		}
		n++
	}
	userSuffixes[user][n] = struct{}{}
	clientID := fmt.Sprintf("%s-%d", user, n)
	return clientID, func() { releaseClientID(user, n) }
}

func releaseClientID(user string, n int) {
	userSuffixesMu.Lock()
	defer userSuffixesMu.Unlock()
	if set, ok := userSuffixes[user]; ok {
		delete(set, n)
		if len(set) == 0 {
			delete(userSuffixes, user)
		}
	}
}

type ServerInfo struct {
	ID     string
	Port   int
	Status string
}

type ServerSession struct {
	Info ServerInfo
	Ssh  *ssh.Server
	PTY  *PTYManager
}

type Daemon struct {
	mu      sync.Mutex
	servers map[string]*ServerSession
}

func NewDaemon() *Daemon {
	return &Daemon{
		servers: make(map[string]*ServerSession),
	}
}

func (d *Daemon) StartServer(req *int, res *ServerInfo) error {
	d.mu.Lock()
	defer d.mu.Unlock()
	port := *req
	for _, sess := range d.servers {
		if sess.Info.Port == port {
			return fmt.Errorf("server already running on port %d", port)
		}
	}

	id := uuid.New().String()[:8]
	pm, err := NewPTYManager(func() {
		d.mu.Lock()
		defer d.mu.Unlock()
		if sess, exists := d.servers[id]; exists {
			sess.Ssh.Close()
			delete(d.servers, id)
		}
	})
	if err != nil {
		return err
	}

	s, err := d.createSshServer(port, id, pm)
	if err != nil {
		pm.Close()
		return err
	}

	d.servers[id] = &ServerSession{
		Info: ServerInfo{ID: id, Port: port, Status: "Active"},
		Ssh:  s,
		PTY:  pm,
	}
	*res = d.servers[id].Info
	return nil
}

func (d *Daemon) createSshServer(port int, id string, pm *PTYManager) (*ssh.Server, error) {
	s, err := wish.NewServer(
		wish.WithAddress(fmt.Sprintf(":%d", port)),
		wish.WithHostKeyPath(fmt.Sprintf(".ssh/term_info_%d_ed25519", port)),
		wish.WithMiddleware(
			func(h ssh.Handler) ssh.Handler {
				return func(s ssh.Session) {
					clientID, release := acquireClientID(s.User())
					defer release()

					wish.Println(s, lipgloss.NewStyle().Foreground(lipgloss.Color("99")).Render("\n"+GhostArt))
					wish.Println(s, "🌈 Welcome to Wisp! 🌈")
					wish.Printf(s, "Session ID: %s\n", id)
					wish.Println(s, "Authenticated as:", clientID)

					ptyReq, winCh, isPty := s.Pty()
					if !isPty {
						wish.Println(s, "No PTY requested")
						return
					}

					remote := ""
					if addr := s.RemoteAddr(); addr != nil {
						remote = addr.String()
					}
					pm.Attach(s, clientID, remote, ptyReq.Window)
					go func() {
						for win := range winCh {
							pm.Resize(s, win)
						}
					}()

					pm.HandleSession(s, clientID)
				}
			},
			logging.Middleware(),
		),
	)
	if err != nil {
		return nil, err
	}

	go func() {
		if err = s.ListenAndServe(); err != nil && err != ssh.ErrServerClosed {
			log.Printf("SSH server on %d failed: %v", port, err)
		}
	}()
	log.Printf("Started SSH server on :%d (ID: %s)", port, id)
	return s, nil
}

func (d *Daemon) ListServers(req *int, res *[]ServerInfo) error {
	d.mu.Lock()
	defer d.mu.Unlock()
	infos := make([]ServerInfo, 0, len(d.servers))
	for _, sess := range d.servers {
		infos = append(infos, sess.Info)
	}
	// Sort by port so the order is stable across calls. Map iteration in Go
	// is randomised, which would otherwise spam consumers (CLI / GUI) with
	// "the list changed" signals on every poll.
	sort.Slice(infos, func(i, j int) bool {
		if infos[i].Port != infos[j].Port {
			return infos[i].Port < infos[j].Port
		}
		return infos[i].ID < infos[j].ID
	})
	*res = infos
	return nil
}

func (d *Daemon) KillServer(req *string, res *bool) error {
	d.mu.Lock()
	defer d.mu.Unlock()
	id := *req
	sess, exists := d.servers[id]
	if !exists {
		return fmt.Errorf("session %s not found", id)
	}
	sess.Ssh.Close()
	sess.PTY.Close()
	delete(d.servers, id)
	*res = true
	return nil
}

func (d *Daemon) DownServer(req *string, res *bool) error {
	d.mu.Lock()
	defer d.mu.Unlock()
	id := *req
	sess, exists := d.servers[id]
	if !exists {
		return fmt.Errorf("session %s not found", id)
	}
	if sess.Info.Status == "Down" {
		*res = true
		return nil
	}
	sess.Ssh.Close()
	sess.Info.Status = "Down"
	*res = true
	return nil
}

func (d *Daemon) UpServer(req *string, res *bool) error {
	d.mu.Lock()
	defer d.mu.Unlock()
	id := *req
	sess, exists := d.servers[id]
	if !exists {
		return fmt.Errorf("session %s not found", id)
	}
	if sess.Info.Status == "Active" {
		return fmt.Errorf("session %s is already active", id)
	}

	s, err := d.createSshServer(sess.Info.Port, sess.Info.ID, sess.PTY)
	if err != nil {
		return err
	}
	sess.Ssh = s
	sess.Info.Status = "Active"
	*res = true
	return nil
}

// PeersReq identifies a session whose attached clients should be listed.
type PeersReq struct {
	SessionID string
}

// ListPeers returns the clients currently attached to the given session.
func (d *Daemon) ListPeers(req *PeersReq, res *[]PeerInfo) error {
	d.mu.Lock()
	sess, exists := d.servers[req.SessionID]
	d.mu.Unlock()
	if !exists {
		return fmt.Errorf("session %s not found", req.SessionID)
	}
	*res = sess.PTY.Peers()
	return nil
}

// KickReq targets a single client within a session.
type KickReq struct {
	SessionID string
	ClientID  string
}

// KickPeer terminates the SSH session of a single client within a wisp
// session. Sets the bool result to true on a successful kick, false if the
// client wasn't found.
func (d *Daemon) KickPeer(req *KickReq, res *bool) error {
	d.mu.Lock()
	sess, exists := d.servers[req.SessionID]
	d.mu.Unlock()
	if !exists {
		return fmt.Errorf("session %s not found", req.SessionID)
	}
	*res = sess.PTY.Kick(req.ClientID)
	if !*res {
		return fmt.Errorf("client %s not found in session %s", req.ClientID, req.SessionID)
	}
	return nil
}
