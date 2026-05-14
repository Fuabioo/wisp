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
		configPath, _ := cmd.Flags().GetString("config")
		resolved, err := core.ResolveConfigPath(configPath)
		if err != nil {
			return fmt.Errorf("resolve config path: %w", err)
		}
		cfg, loadErr := core.LoadConfig(resolved)
		if loadErr != nil {
			log.Printf("config load: %v", loadErr)
		}

		d := core.NewDaemon()
		d.Config = cfg
		d.ShadowDir, _ = cmd.Flags().GetString("shadow-dir")
		d.Env = parseEnvFlags(cmd)
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
	daemonCmd.Flags().String("config", "", "Path to config file (defaults to ~/.config/wisp/config.toml)")
	daemonCmd.Flags().String("shadow-dir", "", "Directory to prepend to PATH for all sessions (shadow binaries)")
	daemonCmd.Flags().StringSlice("env", nil, "Environment overrides for all sessions (KEY=VALUE)")
	rootCmd.AddCommand(daemonCmd)
}
