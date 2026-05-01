package core

import (
	"fmt"
	"log"
	"sync"

	"github.com/charmbracelet/lipgloss"
	"github.com/charmbracelet/ssh"
	"github.com/charmbracelet/wish"
	"github.com/charmbracelet/wish/logging"
	"github.com/google/uuid"
)

var (
	userCounts   = make(map[string]int)
	userCountsMu sync.Mutex
)

func getClientID(user string) string {
	userCountsMu.Lock()
	defer userCountsMu.Unlock()
	userCounts[user]++
	return fmt.Sprintf("%s-%d", user, userCounts[user])
}

type ServerInfo struct {
	ID     string
	Port   int
	Status string // "Active", "Down"
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
			return fmt.Errorf("Server already running on port %d", port)
		}
	}

	id := uuid.New().String()[:8]
	pm := NewPTYManager(func() {
		d.mu.Lock()
		defer d.mu.Unlock()
		if sess, exists := d.servers[id]; exists {
			sess.Ssh.Close()
			delete(d.servers, id)
		}
	})

	s, err := d.createSshServer(port, id, pm)
	if err != nil {
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
					clientID := getClientID(s.User())
					wish.Println(s, lipgloss.NewStyle().Foreground(lipgloss.Color("99")).Render("\n"+GhostArt))
					wish.Println(s, "🌈 Welcome to Wisp! 🌈")
					wish.Printf(s, "Session ID: %s\n", id)
					wish.Println(s, "Authenticated as:", clientID)

					ptyReq, winCh, isPty := s.Pty()
					if !isPty {
						wish.Println(s, "No PTY requested")
						return
					}

					pm.Resize(s, ptyReq.Window)
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
	var infos []ServerInfo
	for _, sess := range d.servers {
		infos = append(infos, sess.Info)
	}
	*res = infos
	return nil
}

func (d *Daemon) KillServer(req *string, res *bool) error {
	d.mu.Lock()
	defer d.mu.Unlock()
	id := *req
	sess, exists := d.servers[id]
	if !exists {
		return fmt.Errorf("Session %s not found", id)
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
		return fmt.Errorf("Session %s not found", id)
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
		return fmt.Errorf("Session %s not found", id)
	}
	if sess.Info.Status == "Active" {
		return fmt.Errorf("Session %s is already active", id)
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
