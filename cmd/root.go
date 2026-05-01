package cmd

import (
	"context"
	"fmt"
	"net/rpc"
	"os"

	"wisp/internal/core"

	"github.com/charmbracelet/fang"
	"github.com/charmbracelet/lipgloss"
	"github.com/spf13/cobra"
)

const socketPath = "/tmp/wisp.sock"

var (
	Version   = "dev"
	CommitSHA = "none"
	BuildDate = "unknown"
)

var (
	successStyle = lipgloss.NewStyle().Foreground(lipgloss.Color("212")).Bold(true)
	accentStyle  = lipgloss.NewStyle().Foreground(lipgloss.Color("99"))
)

var rootCmd = &cobra.Command{
	Use:   "wisp",
	Short: "Wisp manages shared terminal sessions",
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
