package cmd

import (
	"github.com/Fuabioo/wisp/internal/core"

	"github.com/spf13/cobra"
)

// tailJSON wraps the raw PTY tail in a named field so JSON consumers
// (notably the COSMIC GUI's CliBackend) get a stable envelope.
type tailJSON struct {
	Tail string `json:"tail"`
}

var tailCmd = &cobra.Command{
	Use:   "tail [uuid]",
	Short: "Print recent PTY output for a Wisp session",
	Long: `tail prints up to ~64 KiB of recent output captured from a session's
PTY. Bytes are emitted as-is, including ANSI escape sequences — pipe
through a viewer if you want a rendered TUI snapshot.`,
	Args: cobra.ExactArgs(1),
	RunE: func(cmd *cobra.Command, args []string) error {
		jsonGuard(cmd)
		client, err := dialDaemon()
		if err != nil {
			return emitFailure(cmd, err)
		}
		defer client.Close()

		var res string
		req := core.TailReq{SessionID: args[0]}
		if err := client.Call("Daemon.GetTail", &req, &res); err != nil {
			return emitFailure(cmd, err)
		}

		if jsonOutput {
			return emitJSON(cmd, tailJSON{Tail: res})
		}
		cmd.Print(res)
		return nil
	},
}

func init() {
	rootCmd.AddCommand(tailCmd)
}
