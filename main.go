package main

import (
	"context"
	"fmt"
	"io"
	"log"
	"net/rpc"
	"os"
	"os/exec"
	"strings"
	"sync"
	"time"

	"github.com/charmbracelet/fang"
	"github.com/charmbracelet/lipgloss"
	"github.com/charmbracelet/lipgloss/table"
	"github.com/charmbracelet/ssh"
	"github.com/creack/pty"
	"github.com/spf13/cobra"
)

type PTYManager struct {
	file    *os.File
	cmd     *exec.Cmd
	mu      sync.Mutex
	socks   map[ssh.Session]ssh.Window
	onClose func()
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
				return n, io.EOF
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

func newPTYManager(onClose func()) *PTYManager {
	shell := os.Getenv("SHELL")
	if shell == "" {
		shell = "zsh" // fallback
	}
	c := exec.Command(shell)
	f, err := pty.Start(c)
	if err != nil {
		log.Fatalf("Failed to start pty: %v", err)
	}

	pm := &PTYManager{
		file:    f,
		cmd:     c,
		socks:   make(map[ssh.Session]ssh.Window),
		onClose: onClose,
	}

	go pm.broadcast()
	return pm
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
		for s := range pm.socks {
			_, err := s.Write(buf[:n])
			if err != nil {
				delete(pm.socks, s)
			}
		}
		pm.mu.Unlock()
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
			for i := 0; i < n; i++ {
				bytesChan <- buf[i]
			}
		}
	}()

	var pendingBang bool
	var timeoutChan <-chan time.Time

	for {
		select {
		case c, ok := <-bytesChan:
			if !ok {
				return
			}
			if pendingBang {
				pendingBang = false
				timeoutChan = nil
				if c == '>' {
					choice, _ := RunMenu(s, chanReader{ch: bytesChan}, clientID)
					if choice == "Disconnect" {
						return
					}
					continue
				} else {
					pm.file.Write([]byte{'!'})
				}
			}

			if c == '!' {
				pendingBang = true
				timeoutChan = time.After(250 * time.Millisecond)
				continue
			}
			pm.file.Write([]byte{c})

		case <-timeoutChan:
			pendingBang = false
			timeoutChan = nil
			pm.file.Write([]byte{'!'})
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
	if !first {
		pty.Setsize(pm.file, &pty.Winsize{
			Rows: minRows,
			Cols: minCols,
		})
	}
}

func (pm *PTYManager) Close() {
	pm.mu.Lock()
	defer pm.mu.Unlock()
	pm.file.Close()
	if pm.cmd != nil && pm.cmd.Process != nil {
		pm.cmd.Process.Kill()
	}
}

var (
	rootCmd = &cobra.Command{
		Use:   "wisp",
		Short: "Wisp manages shared terminal sessions",
	}
	userCounts   = make(map[string]int)
	userCountsMu sync.Mutex

	successStyle = lipgloss.NewStyle().Foreground(lipgloss.Color("212")).Bold(true)
	accentStyle  = lipgloss.NewStyle().Foreground(lipgloss.Color("99"))
)

func getClientID(user string) string {
	userCountsMu.Lock()
	defer userCountsMu.Unlock()
	userCounts[user]++
	return fmt.Sprintf("%s-%d", user, userCounts[user])
}

var serverCmd = &cobra.Command{
	Use:   "server",
	Short: "Start a new SSH server on the specified port",
	RunE: func(cmd *cobra.Command, args []string) error {
		port, _ := cmd.Flags().GetInt("port")
		
		client, err := rpc.Dial("unix", "/tmp/wisp.sock")
		if err != nil {
			return fmt.Errorf("Could not connect to daemon (is it running?): %v", err)
		}
		defer client.Close()

		var res ServerInfo
		err = client.Call("Daemon.StartServer", &port, &res)
		if err != nil {
			return err
		}
		cmd.Println(successStyle.Render(fmt.Sprintf("👻 Successfully started server on port %d ", port)) + accentStyle.Render(fmt.Sprintf("(ID: %s)", res.ID)))
		return nil
	},
}

var psCmd = &cobra.Command{
	Use:   "ps",
	Short: "List running Wisp servers",
	RunE: func(cmd *cobra.Command, args []string) error {
		client, err := rpc.Dial("unix", "/tmp/wisp.sock")
		if err != nil {
			return fmt.Errorf("Could not connect to daemon: %v", err)
		}
		defer client.Close()

		var res []ServerInfo
		err = client.Call("Daemon.ListServers", 0, &res)
		if err != nil {
			return err
		}

		if len(res) == 0 {
			cmd.Println(lipgloss.NewStyle().Foreground(lipgloss.Color("204")).Italic(true).Render("No Wisp servers currently running. 👻"))
			return nil
		}

		rows := make([][]string, 0, len(res))
		for _, info := range res {
			rows = append(rows, []string{info.ID, fmt.Sprintf("%d", info.Port), info.Status, fmt.Sprintf("ssh -p %d localhost", info.Port)})
		}

		t := table.New().
			Border(lipgloss.NormalBorder()).
			BorderStyle(accentStyle).
			Headers("ID", "PORT", "STATUS", "CONNECT COMMAND").
			Rows(rows...)

		cmd.Println(accentStyle.Render("\n🌈 Running Wisp Servers:\n"))
		cmd.Println(t)
		return nil
	},
}

var killCmd = &cobra.Command{
	Use:   "kill [uuid]",
	Short: "Kill a running Wisp server by UUID",
	Args:  cobra.ExactArgs(1),
	RunE: func(cmd *cobra.Command, args []string) error {
		client, err := rpc.Dial("unix", "/tmp/wisp.sock")
		if err != nil {
			return fmt.Errorf("Could not connect to daemon: %v", err)
		}
		defer client.Close()

		var res bool
		err = client.Call("Daemon.KillServer", &args[0], &res)
		if err != nil {
			return err
		}
		cmd.Println(successStyle.Render("💀 Successfully killed server ") + accentStyle.Render(args[0]))
		return nil
	},
}

var downCmd = &cobra.Command{
	Use:   "down [uuid]",
	Short: "Deactivate a running Wisp server by UUID",
	Args:  cobra.ExactArgs(1),
	RunE: func(cmd *cobra.Command, args []string) error {
		client, err := rpc.Dial("unix", "/tmp/wisp.sock")
		if err != nil {
			return fmt.Errorf("Could not connect to daemon: %v", err)
		}
		defer client.Close()

		var res bool
		err = client.Call("Daemon.DownServer", &args[0], &res)
		if err != nil {
			return err
		}
		cmd.Println(successStyle.Render("💤 Successfully brought down server ") + accentStyle.Render(args[0]))
		return nil
	},
}

var upCmd = &cobra.Command{
	Use:   "up [uuid]",
	Short: "Reactivate a down Wisp server by UUID",
	Args:  cobra.ExactArgs(1),
	RunE: func(cmd *cobra.Command, args []string) error {
		client, err := rpc.Dial("unix", "/tmp/wisp.sock")
		if err != nil {
			return fmt.Errorf("Could not connect to daemon: %v", err)
		}
		defer client.Close()

		var res bool
		err = client.Call("Daemon.UpServer", &args[0], &res)
		if err != nil {
			return err
		}
		cmd.Println(successStyle.Render("✨ Successfully brought up server ") + accentStyle.Render(args[0]))
		return nil
	},
}

func main() {
	serverCmd.Flags().IntP("port", "p", 2222, "Port to listen on")
	rootCmd.AddCommand(serverCmd)
	rootCmd.AddCommand(daemonCmd)
	rootCmd.AddCommand(psCmd)
	rootCmd.AddCommand(killCmd)
	rootCmd.AddCommand(downCmd)
	rootCmd.AddCommand(upCmd)

	if err := fang.Execute(context.Background(), rootCmd); err != nil {
		os.Exit(1)
	}
}
