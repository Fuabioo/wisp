package cmd

import (
	"encoding/json"
	"strings"

	"github.com/spf13/cobra"
)

// jsonOutput is set by the persistent --json flag (registered in root.go).
// When true, every command emits a stable, machine-readable JSON document on
// stdout in place of the styled human output. Used by the COSMIC admin GUI's
// CliBackend (see docs/adr/0002-cosmic-admin-gui.md).
var jsonOutput bool

// parseEnvFlags reads the --env flag (repeatable, KEY=VALUE) from the given
// cobra command and returns a map. Used by both the daemon (global defaults)
// and server (per-session overrides) subcommands.
func parseEnvFlags(cmd *cobra.Command) map[string]string {
	raw, _ := cmd.Flags().GetStringSlice("env")
	out := make(map[string]string, len(raw))
	for _, kv := range raw {
		k, v, ok := strings.Cut(kv, "=")
		if !ok {
			continue
		}
		out[k] = v
	}
	return out
}

// jsonResult is the envelope returned by action commands (server, up, down,
// kill, kick). Read commands (ps, peers) emit their typed payload directly.
type jsonResult struct {
	OK    bool   `json:"ok"`
	ID    string `json:"id,omitempty"`
	Port  int    `json:"port,omitempty"`
	Error string `json:"error,omitempty"`
}

func emitJSON(cmd *cobra.Command, v any) error {
	return json.NewEncoder(cmd.OutOrStdout()).Encode(v)
}

// jsonGuard silences cobra's own error/usage printing so the JSON document is
// the only thing that lands on stdout/stderr.
func jsonGuard(cmd *cobra.Command) {
	if jsonOutput {
		cmd.SilenceErrors = true
		cmd.SilenceUsage = true
	}
}

// emitFailure writes a JSON error envelope when --json is in effect, then
// returns err so the process still exits non-zero.
func emitFailure(cmd *cobra.Command, err error) error {
	if jsonOutput {
		_ = emitJSON(cmd, jsonResult{Error: err.Error()})
	}
	return err
}

// emitSuccess routes either to the styled human output or to a JSON success
// envelope depending on the --json flag. The OK field is set automatically.
func emitSuccess(cmd *cobra.Command, payload jsonResult, humanOutput string) error {
	if jsonOutput {
		payload.OK = true
		return emitJSON(cmd, payload)
	}
	cmd.Println(humanOutput)
	return nil
}
