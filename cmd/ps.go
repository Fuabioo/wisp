package cmd

import (
	"fmt"
	"net/rpc"
	"wisp/internal/core"

	"github.com/charmbracelet/lipgloss"
	"github.com/charmbracelet/lipgloss/table"
	"github.com/spf13/cobra"
)

var psCmd = &cobra.Command{
	Use:   "ps",
	Short: "List running Wisp servers",
	RunE: func(cmd *cobra.Command, args []string) error {
		client, err := rpc.Dial("unix", "/tmp/wisp.sock")
		if err != nil {
			return fmt.Errorf("Could not connect to daemon: %v", err)
		}
		defer client.Close()

		var res []core.ServerInfo
		err = client.Call("Daemon.ListServers", 0, &res)
		if err != nil {
			return err
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
