package cmd

import (
	"github.com/spf13/cobra"
)

var downCmd = &cobra.Command{
	Use:   "down [uuid]",
	Short: "Deactivate a running Wisp server by UUID",
	Args:  cobra.ExactArgs(1),
	RunE: func(cmd *cobra.Command, args []string) error {
		jsonGuard(cmd)
		client, err := dialDaemon()
		if err != nil {
			return emitFailure(cmd, err)
		}
		defer client.Close()

		var res bool
		if err := client.Call("Daemon.DownServer", &args[0], &res); err != nil {
			return emitFailure(cmd, err)
		}

		return emitSuccess(cmd,
			jsonResult{ID: args[0]},
			successStyle.Render("💤 Successfully brought down server ")+accentStyle.Render(args[0]))
	},
}

func init() {
	rootCmd.AddCommand(downCmd)
}
