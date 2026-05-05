package cmd

import (
	"github.com/Fuabioo/wisp/internal/core"

	"github.com/spf13/cobra"
)

var refreshCmd = &cobra.Command{
	Use:   "refresh [uuid]",
	Short: "Nudge a session's PTY size to coax TUIs into repainting",
	Long: `refresh perturbs the PTY size by +1 and back, generating a SIGWINCH
on the foreground process. Full-screen TUIs (claude-code, vim, htop, …)
repaint on resize, so this gives a peer who attached mid-session a fresh
paint without anyone disconnecting.`,
	Args: cobra.ExactArgs(1),
	RunE: func(cmd *cobra.Command, args []string) error {
		jsonGuard(cmd)
		client, err := dialDaemon()
		if err != nil {
			return emitFailure(cmd, err)
		}
		defer client.Close()

		var res bool
		req := core.RefreshReq{SessionID: args[0]}
		if err := client.Call("Daemon.RefreshSession", &req, &res); err != nil {
			return emitFailure(cmd, err)
		}

		return emitSuccess(cmd,
			jsonResult{ID: args[0]},
			successStyle.Render("🔁 Refreshed TUI for session ")+accentStyle.Render(args[0]))
	},
}

func init() {
	rootCmd.AddCommand(refreshCmd)
}
