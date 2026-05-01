package main

import (
	"fmt"
	"io"
	"log"
	"os"
	"os/exec"
	"os/signal"
	"strings"
	"sync"
	"syscall"

	"github.com/charmbracelet/ssh"
	"github.com/charmbracelet/wish"
	"github.com/charmbracelet/wish/logging"
	"github.com/creack/pty"
	"github.com/spf13/cobra"
)

type PTYManager struct {
	file  *os.File
	cmd   *exec.Cmd
	mu    sync.Mutex
	socks map[ssh.Session]ssh.Window
}

func newPTYManager() *PTYManager {
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
		file:  f,
		cmd:   c,
		socks: make(map[ssh.Session]ssh.Window),
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
				os.Exit(0)
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

func (pm *PTYManager) HandleSession(s ssh.Session) {
	defer func() {
		pm.mu.Lock()
		delete(pm.socks, s)
		pm.updateSizeLocked()
		pm.mu.Unlock()
	}()

	// Read client input and send to PTY
	io.Copy(pm.file, s)
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

var rootCmd = &cobra.Command{
	Use:   "prism",
	Short: "Prism manages shared terminal sessions",
}

var serverCmd = &cobra.Command{
	Use:   "server",
	Short: "Start the Prism SSH daemon",
	RunE: func(cmd *cobra.Command, args []string) error {
		port, _ := cmd.Flags().GetInt("port")
		
		pm := newPTYManager()

		s, err := wish.NewServer(
			wish.WithAddress(fmt.Sprintf(":%d", port)),
			wish.WithHostKeyPath(".ssh/term_info_ed25519"),
			wish.WithMiddleware(
				func(h ssh.Handler) ssh.Handler {
					return func(s ssh.Session) {
						wish.Println(s, "🌈 Welcome to Prism! 🌈")
						wish.Println(s, "Authenticated as:", s.User())
						
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

						pm.HandleSession(s)
					}
				},
				logging.Middleware(),
			),
		)
		if err != nil {
			return err
		}

		done := make(chan os.Signal, 1)
		signal.Notify(done, os.Interrupt, syscall.SIGINT, syscall.SIGTERM)

		log.Printf("Starting SSH server on :%d", port)
		go func() {
			if err = s.ListenAndServe(); err != nil {
				log.Fatal(err)
			}
		}()

		<-done
		log.Println("Stopping SSH server")
		return s.Close()
	},
}

func main() {
	serverCmd.Flags().IntP("port", "p", 2222, "Port to listen on")
	rootCmd.AddCommand(serverCmd)

	if err := rootCmd.Execute(); err != nil {
		fmt.Println(err)
		os.Exit(1)
	}
}
