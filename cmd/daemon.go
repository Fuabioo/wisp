package cmd

import (
	"fmt"
	"log"
	"net"
	"net/rpc"
	"os"
	"os/signal"
	"syscall"

	"github.com/spf13/cobra"
	"wisp/internal/core"
)

var daemonCmd = &cobra.Command{
	Use:   "daemon",
	Short: "Start the Wisp management daemon",
	RunE: func(cmd *cobra.Command, args []string) error {
		d := core.NewDaemon()
		rpc.Register(d)

		os.Remove("/tmp/wisp.sock")
		l, err := net.Listen("unix", "/tmp/wisp.sock")
		if err != nil {
			return err
		}
		defer l.Close()

		go rpc.Accept(l)
		
		fmt.Println(accentStyle.Render(`
   ▄██████▄ 
  ██▄▀██▀▄██
  ████▄▄████
   ▀█▄▀▄█▀  
`))
		log.Println("Wisp daemon started on /tmp/wisp.sock")

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
