package cmd

import (
	"fmt"

	"github.com/Fuabioo/wisp/internal/core"

	"github.com/spf13/cobra"
)

var serverCmd = &cobra.Command{
	Use:   "server",
	Short: "Start a new SSH server on the specified port",
	RunE: func(cmd *cobra.Command, args []string) error {
		jsonGuard(cmd)
		port, _ := cmd.Flags().GetInt("port")
		shell, _ := cmd.Flags().GetString("shell")
		shadowDir, _ := cmd.Flags().GetString("shadow-dir")

		client, err := dialDaemon()
		if err != nil {
			return emitFailure(cmd, err)
		}
		defer client.Close()

		var res core.ServerInfo
		req := core.StartServerReq{
			Port:      port,
			Shell:     shell,
			ShadowDir: shadowDir,
			Env:       parseEnvFlags(cmd),
		}
		if err := client.Call("Daemon.StartServer", &req, &res); err != nil {
			return emitFailure(cmd, err)
		}

		return emitSuccess(cmd,
			jsonResult{ID: res.ID, Port: res.Port},
			successStyle.Render(fmt.Sprintf("👻 Successfully started server on port %d ", port))+
				accentStyle.Render(fmt.Sprintf("(ID: %s)", res.ID)))
	},
}

func init() {
	serverCmd.Flags().IntP("port", "p", 2222, "Port to listen on")
	serverCmd.Flags().StringP("shell", "s", "", "Shell binary (defaults to $SHELL, then zsh)")
	serverCmd.Flags().String("shadow-dir", "", "Prepend this directory to PATH inside the PTY (shadow binaries)")
	serverCmd.Flags().StringSlice("env", nil, "Env overrides for the PTY (KEY=VALUE, repeatable)")
	rootCmd.AddCommand(serverCmd)
}
