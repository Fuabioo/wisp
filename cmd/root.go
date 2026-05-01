package cmd

import (
	"context"
	"fmt"
	"os"

	"wisp/internal/core"

	"github.com/charmbracelet/fang"
	"github.com/charmbracelet/lipgloss"
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

var rootCmd = &cobra.Command{
	Use:   "wisp",
	Short: "Wisp manages shared terminal sessions",
}

func Execute() {
	versionStr := fmt.Sprintf("\n%s\n%s %s %s", accentStyle.Render(core.GhostArt), Version, CommitSHA, BuildDate)
	if err := fang.Execute(context.Background(), rootCmd, fang.WithVersion(versionStr)); err != nil {
		os.Exit(1)
	}
}
