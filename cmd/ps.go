package cmd

import (
	"fmt"

	"github.com/Fuabioo/wisp/internal/core"

	"charm.land/lipgloss/v2"
	"charm.land/lipgloss/v2/table"
	"github.com/spf13/cobra"
)

var psCmd = &cobra.Command{
	Use:   "ps",
	Short: "List running Wisp servers",
	RunE: func(cmd *cobra.Command, args []string) error {
		jsonGuard(cmd)
		client, err := dialDaemon()
		if err != nil {
			return emitFailure(cmd, err)
		}
		defer client.Close()

		var res []core.ServerInfo
		if err := client.Call("Daemon.ListServers", 0, &res); err != nil {
			return emitFailure(cmd, err)
		}

		if jsonOutput {
			if res == nil {
				res = []core.ServerInfo{}
			}
			return emitJSON(cmd, res)
		}

		if len(res) == 0 {
			cmd.Println(lipgloss.NewStyle().Foreground(lipgloss.Color("204")).Italic(true).Render("No Wisp servers currently running. 👻"))
			return nil
		}

		rows := make([][]string, 0, len(res))
		for _, info := range res {
			rows = append(rows, []string{info.ID, fmt.Sprintf("%d", info.Port), info.Status, fmt.Sprintf("ssh -p %d localhost", info.Port)})
		}

		t := table.New().
			Border(lipgloss.NormalBorder()).
			BorderStyle(accentStyle).
			Headers("ID", "PORT", "STATUS", "CONNECT COMMAND").
			Rows(rows...)

		cmd.Println(accentStyle.Render("\n🌈 Running Wisp Servers:\n"))
		cmd.Println(t)
		return nil
	},
}

func init() {
	rootCmd.AddCommand(psCmd)
}
