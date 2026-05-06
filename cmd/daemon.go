package cmd

import (
	"fmt"
	"log"
	"net"
	"net/rpc"
	"os"
	"os/signal"
	"syscall"

	"github.com/Fuabioo/wisp/internal/core"

	"github.com/spf13/cobra"
)

var daemonCmd = &cobra.Command{
	Use:   "daemon",
	Short: "Start the Wisp management daemon",
	RunE: func(cmd *cobra.Command, args []string) error {
		d := core.NewDaemon()
		if err := rpc.Register(d); err != nil {
			return fmt.Errorf("register rpc: %w", err)
		}

		os.Remove(socketPath)
		l, err := net.Listen("unix", socketPath)
		if err != nil {
			return err
		}
		defer l.Close()

		go rpc.Accept(l)

		fmt.Println(accentStyle.Render("\n" + core.GhostArt + "\n"))
		fmt.Printf("%s %s %s\n", Version, CommitSHA, BuildDate)
		log.Printf("Wisp daemon started on %s", socketPath)

		done := make(chan os.Signal, 1)
		signal.Notify(done, os.Interrupt, syscall.SIGINT, syscall.SIGTERM)
		<-done
		log.Println("Stopping daemon")
		return nil
	},
}

func init() {
	rootCmd.AddCommand(daemonCmd)
}
