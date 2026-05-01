package cmd

import (
	"github.com/spf13/cobra"
)

var upCmd = &cobra.Command{
	Use:   "up [uuid]",
	Short: "Reactivate a down Wisp server by UUID",
	Args:  cobra.ExactArgs(1),
	RunE: func(cmd *cobra.Command, args []string) error {
		client, err := dialDaemon()
		if err != nil {
			return err
		}
		defer client.Close()

		var res bool
		if err := client.Call("Daemon.UpServer", &args[0], &res); err != nil {
			return err
		}
		cmd.Println(successStyle.Render("✨ Successfully brought up server ") + accentStyle.Render(args[0]))
		return nil
	},
}

func init() {
	rootCmd.AddCommand(upCmd)
}
