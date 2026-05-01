package cmd

import (
	"fmt"
	"net/rpc"

	"github.com/spf13/cobra"
)

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

func init() {
	rootCmd.AddCommand(killCmd)
}
