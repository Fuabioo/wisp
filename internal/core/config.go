package core

import (
	"fmt"
	"os"
	"path/filepath"

	"github.com/BurntSushi/toml"
)

// ThemeVariant holds the colours for one side of the light/dark split.
type ThemeVariant struct {
	PrimaryBG    string `toml:"primary_bg"`
	PrimaryFG    string `toml:"primary_fg"`
	SuggestionFG string `toml:"suggestion_fg"`
}

// ThemeConfig holds the dark and light colour palettes.
type ThemeConfig struct {
	Dark  ThemeVariant `toml:"dark"`
	Light ThemeVariant `toml:"light"`
}

// StatusBarConfig controls the per-session status bar overlay.
type StatusBarConfig struct {
	Enabled    bool   `toml:"enabled"`
	Position   string `toml:"position"`
	Suggestion string `toml:"suggestion"`
}

// Config is the root daemon configuration, loaded from
// ~/.config/wisp/config.toml (XDG-compliant).
type Config struct {
	Theme     ThemeConfig     `toml:"theme"`
	StatusBar StatusBarConfig `toml:"status_bar"`
}

// DefaultConfig returns a Config populated with the canonical brand
// palette and sensible defaults so the daemon works without a config file.
func DefaultConfig() Config {
	return Config{
		Theme: ThemeConfig{
			Dark: ThemeVariant{
				PrimaryBG:    "#9B6EFF",
				PrimaryFG:    "#FFFFFF",
				SuggestionFG: "#888888",
			},
			Light: ThemeVariant{
				PrimaryBG:    "#9B6EFF",
				PrimaryFG:    "#111111",
				SuggestionFG: "#666666",
			},
		},
		StatusBar: StatusBarConfig{
			Enabled:    true,
			Position:   "top",
			Suggestion: "Type !> for pause menu",
		},
	}
}

// ResolvePath returns the canonical config file path. If the user
// supplied --config, it's used directly. Otherwise we look at
// XDG_CONFIG_HOME / $HOME/.config.
func ResolveConfigPath(explicit string) (string, error) {
	if explicit != "" {
		return explicit, nil
	}
	dir := os.Getenv("XDG_CONFIG_HOME")
	if dir == "" {
		home, err := os.UserHomeDir()
		if err != nil {
			return "", fmt.Errorf("resolve home dir for config: %w", err)
		}
		dir = filepath.Join(home, ".config")
	}
	return filepath.Join(dir, "wisp", "config.toml"), nil
}

// LoadConfig reads and decodes the TOML config file at the given path.
// If the file is missing or unparseable, returns DefaultConfig() with
// the error logged but not fatal — the daemon runs with safe defaults.
func LoadConfig(path string) (Config, error) {
	cfg := DefaultConfig()
	data, err := os.ReadFile(path)
	if err != nil {
		return cfg, fmt.Errorf("reading config %s: %w (using defaults)", path, err)
	}
	if err := toml.Unmarshal(data, &cfg); err != nil {
		return DefaultConfig(), fmt.Errorf("parsing config %s: %w (using defaults)", path, err)
	}
	if cfg.Theme.Dark.PrimaryBG == "" {
		cfg.Theme.Dark = DefaultConfig().Theme.Dark
	}
	if cfg.Theme.Light.PrimaryBG == "" {
		cfg.Theme.Light = DefaultConfig().Theme.Light
	}
	if cfg.StatusBar.Position != "top" && cfg.StatusBar.Position != "bottom" {
		cfg.StatusBar.Position = "top"
	}
	return cfg, nil
}
