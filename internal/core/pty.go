package core

import (
	"fmt"
	"io"
	"log"
	"os"
	"os/exec"
	"strings"
	"sync"
	"time"

	"github.com/charmbracelet/ssh"
	"github.com/creack/pty"
)

type PTYManager struct {
	file     *os.File
	cmd      *exec.Cmd
	mu       sync.Mutex
	socks    map[ssh.Session]ssh.Window
	onClose  func()
	lastRows uint16
	lastCols uint16
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

func NewPTYManager(onClose func()) (*PTYManager, error) {
	shell := os.Getenv("SHELL")
	if shell == "" {
		shell = "zsh"
	}
	c := exec.Command(shell)
	f, err := pty.Start(c)
	if err != nil {
		return nil, fmt.Errorf("start pty: %w", err)
	}

	pm := &PTYManager{
		file:    f,
		cmd:     c,
		socks:   make(map[ssh.Session]ssh.Window),
		onClose: onClose,
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

		pm.mu.Lock()
		sessions := make([]ssh.Session, 0, len(pm.socks))
		for s := range pm.socks {
			sessions = append(sessions, s)
		}
		pm.mu.Unlock()

		var dead []ssh.Session
		for _, s := range sessions {
			if _, err := s.Write(buf[:n]); err != nil {
				dead = append(dead, s)
			}
		}
		if len(dead) > 0 {
			pm.mu.Lock()
			for _, s := range dead {
				delete(pm.socks, s)
			}
			pm.mu.Unlock()
		}
	}
}

func (pm *PTYManager) HandleSession(s ssh.Session, clientID string) {
	defer func() {
		pm.mu.Lock()
		delete(pm.socks, s)
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
		if _, err := pm.file.Write([]byte{b}); err != nil {
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
					choice, _ := RunMenu(s, chanReader{ch: bytesChan}, clientID)
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

func (pm *PTYManager) Resize(s ssh.Session, win ssh.Window) {
	pm.mu.Lock()
	defer pm.mu.Unlock()
	pm.socks[s] = win
	pm.updateSizeLocked()
}

func (pm *PTYManager) updateSizeLocked() {
	if len(pm.socks) == 0 {
		return
	}
	var minRows, minCols uint16
	first := true
	for _, win := range pm.socks {
		if win.Width == 0 || win.Height == 0 {
			continue
		}
		if first {
			minRows = uint16(win.Height)
			minCols = uint16(win.Width)
			first = false
		} else {
			if uint16(win.Height) < minRows {
				minRows = uint16(win.Height)
			}
			if uint16(win.Width) < minCols {
				minCols = uint16(win.Width)
			}
		}
	}
	if first {
		return
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

func (pm *PTYManager) Close() {
	pm.mu.Lock()
	defer pm.mu.Unlock()
	pm.file.Close()
	if pm.cmd != nil && pm.cmd.Process != nil {
		pm.cmd.Process.Kill()
	}
}
