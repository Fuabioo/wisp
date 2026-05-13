package cmd

import (
	"fmt"

	"github.com/Fuabioo/wisp/internal/core"

	"charm.land/lipgloss/v2"
	"charm.land/lipgloss/v2/table"
	"github.com/spf13/cobra"
)

var statsCmd = &cobra.Command{
	Use:   "stats [session-id]",
	Short: "Show bandwidth consumption for a Wisp session",
	Long: `stats reports the total bytes sent (PTY → clients) and received
(clients → PTY) for the given session since it was spawned.`,
	Args: cobra.ExactArgs(1),
	RunE: func(cmd *cobra.Command, args []string) error {
		jsonGuard(cmd)
		client, err := dialDaemon()
		if err != nil {
			return emitFailure(cmd, err)
		}
		defer client.Close()

		var res core.SessionStats
		if err := client.Call("Daemon.GetSessionStats", &args[0], &res); err != nil {
			return emitFailure(cmd, err)
		}

		if jsonOutput {
			return emitJSON(cmd, res)
		}

		rows := [][]string{
			{"Download (PTY→clients)", humanizeBytes(res.BytesOut)},
			{"Upload   (clients→PTY)", humanizeBytes(res.BytesIn)},
		}
		t := table.New().
			Border(lipgloss.NormalBorder()).
			BorderStyle(accentStyle).
			Headers("DIRECTION", "TOTAL").
			Rows(rows...)

		cmd.Println(accentStyle.Render(fmt.Sprintf("\n📊 Stats for session %s:\n", args[0])))
		cmd.Println(t)
		return nil
	},
}

func humanizeBytes(n uint64) string {
	const unit = 1024
	if n < unit {
		return fmt.Sprintf("%d B", n)
	}
	div, exp := uint64(unit), 0
	for b := n / unit; b >= unit; b /= unit {
		div *= unit
		exp++
	}
	return fmt.Sprintf("%.1f %cB", float64(n)/float64(div), "KMGTPE"[exp])
}

func init() {
	rootCmd.AddCommand(statsCmd)
}
