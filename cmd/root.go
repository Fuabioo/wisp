package cmd

import (
	"context"
	"os"

	"github.com/charmbracelet/fang"
	"github.com/charmbracelet/lipgloss"
	"github.com/spf13/cobra"
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
	if err := fang.Execute(context.Background(), rootCmd); err != nil {
		os.Exit(1)
	}
}
