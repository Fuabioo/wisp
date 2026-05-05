package cmd

import (
	"fmt"

	"github.com/Fuabioo/wisp/internal/core"

	"github.com/spf13/cobra"
)

var kickCmd = &cobra.Command{
	Use:   "kick [session-id] [client-id]",
	Short: "Disconnect a single client from a Wisp session",
	Args:  cobra.ExactArgs(2),
	RunE: func(cmd *cobra.Command, args []string) error {
		client, err := dialDaemon()
		if err != nil {
			return err
		}
		defer client.Close()

		var res bool
		req := core.KickReq{SessionID: args[0], ClientID: args[1]}
		if err := client.Call("Daemon.KickPeer", &req, &res); err != nil {
			return err
		}
		cmd.Println(successStyle.Render(fmt.Sprintf("👢 Kicked %s from session %s", args[1], args[0])))
		return nil
	},
}

func init() {
	rootCmd.AddCommand(kickCmd)
}
