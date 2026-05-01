package cmd

import (
	"fmt"
	"net/rpc"
	"wisp/internal/core"

	"github.com/spf13/cobra"
)

var serverCmd = &cobra.Command{
	Use:   "server",
	Short: "Start a new SSH server on the specified port",
	RunE: func(cmd *cobra.Command, args []string) error {
		port, _ := cmd.Flags().GetInt("port")

		client, err := rpc.Dial("unix", "/tmp/wisp.sock")
		if err != nil {
			return fmt.Errorf("Could not connect to daemon (is it running?): %v", err)
		}
		defer client.Close()

		var res core.ServerInfo
		err = client.Call("Daemon.StartServer", &port, &res)
		if err != nil {
			return err
		}
		cmd.Println(successStyle.Render(fmt.Sprintf("👻 Successfully started server on port %d ", port)) + accentStyle.Render(fmt.Sprintf("(ID: %s)", res.ID)))
		return nil
	},
}

func init() {
	serverCmd.Flags().IntP("port", "p", 2222, "Port to listen on")
	rootCmd.AddCommand(serverCmd)
}
