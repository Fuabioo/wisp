package cmd

import (
	"context"
	"fmt"
	"net/rpc"
	"os"
	"path/filepath"

	"github.com/Fuabioo/wisp/internal/core"

	"charm.land/fang/v2"
	"charm.land/lipgloss/v2"
	"github.com/spf13/cobra"
)

var (
	Version   = "dev"
	CommitSHA = "none"
	BuildDate = "unknown"
)

var (
	successStyle = lipgloss.NewStyle().Foreground(lipgloss.Color("212")).Bold(true)
	accentStyle  = lipgloss.NewStyle().Foreground(lipgloss.Color("99"))
)

// socketPath is set via the --socket persistent flag (default sourced from
// WISP_SOCKET env, then $XDG_RUNTIME_DIR/wisp.sock, then /tmp/wisp.sock).
var socketPath string

var rootCmd = &cobra.Command{
	Use:   "wisp",
	Short: "Wisp manages shared terminal sessions",
}

func defaultSocketPath() string {
	if v := os.Getenv("WISP_SOCKET"); v != "" {
		return v
	}
	if r := os.Getenv("XDG_RUNTIME_DIR"); r != "" {
		return filepath.Join(r, "wisp.sock")
	}
	return "/tmp/wisp.sock"
}

func init() {
	rootCmd.PersistentFlags().StringVar(&socketPath, "socket", defaultSocketPath(), "Path to the wisp daemon Unix socket")
	rootCmd.PersistentFlags().BoolVar(&jsonOutput, "json", false, "Emit machine-readable JSON instead of styled output")
}

func dialDaemon() (*rpc.Client, error) {
	c, err := rpc.Dial("unix", socketPath)
	if err != nil {
		return nil, fmt.Errorf("could not connect to daemon (is it running?): %w", err)
	}
	return c, nil
}

func Execute() {
	versionStr := fmt.Sprintf("\n%s\n%s %s %s", accentStyle.Render(core.GhostArt), Version, CommitSHA, BuildDate)
	if err := fang.Execute(context.Background(), rootCmd, fang.WithVersion(versionStr)); err != nil {
		os.Exit(1)
	}
}
