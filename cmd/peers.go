package cmd

import (
	"fmt"
	"time"

	"github.com/Fuabioo/wisp/internal/core"

	"charm.land/lipgloss/v2"
	"charm.land/lipgloss/v2/table"
	"github.com/spf13/cobra"
)

var peersCmd = &cobra.Command{
	Use:   "peers [session-id]",
	Short: "List clients attached to a Wisp session",
	Args:  cobra.ExactArgs(1),
	RunE: func(cmd *cobra.Command, args []string) error {
		jsonGuard(cmd)
		client, err := dialDaemon()
		if err != nil {
			return emitFailure(cmd, err)
		}
		defer client.Close()

		var res []core.PeerInfo
		if err := client.Call("Daemon.ListPeers", &core.PeersReq{SessionID: args[0]}, &res); err != nil {
			return emitFailure(cmd, err)
		}

		if jsonOutput {
			if res == nil {
				res = []core.PeerInfo{}
			}
			return emitJSON(cmd, res)
		}

		if len(res) == 0 {
			cmd.Println(lipgloss.NewStyle().Foreground(lipgloss.Color("204")).Italic(true).Render("No peers currently attached. 👻"))
			return nil
		}

		now := time.Now()
		rows := make([][]string, 0, len(res))
		for _, p := range res {
			rows = append(rows, []string{
				p.ClientID,
				fmt.Sprintf("%dx%d", p.Width, p.Height),
				p.RemoteAddr,
				humanizeAge(now.Sub(p.ConnectedAt)),
			})
		}

		t := table.New().
			Border(lipgloss.NormalBorder()).
			BorderStyle(accentStyle).
			Headers("CLIENT", "WINDOW", "REMOTE ADDR", "ATTACHED").
			Rows(rows...)

		cmd.Println(accentStyle.Render(fmt.Sprintf("\n🌈 Peers in session %s:\n", args[0])))
		cmd.Println(t)
		return nil
	},
}

func humanizeAge(d time.Duration) string {
	if d < time.Minute {
		return fmt.Sprintf("%ds", int(d.Seconds()))
	}
	if d < time.Hour {
		return fmt.Sprintf("%dm", int(d.Minutes()))
	}
	return fmt.Sprintf("%dh%dm", int(d.Hours()), int(d.Minutes())%60)
}

func init() {
	rootCmd.AddCommand(peersCmd)
}
