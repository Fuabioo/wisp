package core

import (
	"testing"
	"time"
)

func TestHumanizeDuration_Seconds(t *testing.T) {
	tests := []struct {
		d    time.Duration
		want string
	}{
		{0, "0s"},
		{1 * time.Second, "1s"},
		{30 * time.Second, "30s"},
		{59 * time.Second, "59s"},
	}
	for _, tc := range tests {
		if got := humanizeDuration(tc.d); got != tc.want {
			t.Errorf("humanizeDuration(%v) = %q, want %q", tc.d, got, tc.want)
		}
	}
}

func TestHumanizeDuration_Minutes(t *testing.T) {
	tests := []struct {
		d    time.Duration
		want string
	}{
		{1 * time.Minute, "1m"},
		{90 * time.Second, "1m"},
		{5*time.Minute + 30*time.Second, "5m"},
		{59*time.Minute + 59*time.Second, "59m"},
	}
	for _, tc := range tests {
		if got := humanizeDuration(tc.d); got != tc.want {
			t.Errorf("humanizeDuration(%v) = %q, want %q", tc.d, got, tc.want)
		}
	}
}

func TestHumanizeDuration_Hours(t *testing.T) {
	tests := []struct {
		d    time.Duration
		want string
	}{
		{1 * time.Hour, "1h0m"},
		{90 * time.Minute, "1h30m"},
		{25*time.Hour + 10*time.Minute, "25h10m"},
	}
	for _, tc := range tests {
		if got := humanizeDuration(tc.d); got != tc.want {
			t.Errorf("humanizeDuration(%v) = %q, want %q", tc.d, got, tc.want)
		}
	}
}

func TestMenuModel_Init(t *testing.T) {
	m := MenuModel{}
	cmd := m.Init()
	if cmd != nil {
		t.Fatal("Init should return nil")
	}
}

func TestMenuModel_MenuNavigation(t *testing.T) {
	m := MenuModel{
		clientID: "alice-1",
		items: []menuItem{
			{label: "Resume", action: actionResume},
			{label: "List peers", action: actionShowPeers},
			{label: "Disconnect", action: actionDisconnect},
		},
	}

	if m.cursor != 0 {
		t.Fatalf("initial cursor = %d, want 0", m.cursor)
	}
	if m.state != viewMenu {
		t.Fatal("initial state should be viewMenu")
	}
}

func TestMenuModel_MenuRender(t *testing.T) {
	m := MenuModel{
		clientID: "test-1",
		items: []menuItem{
			{label: "Resume", action: actionResume},
		},
	}
	view := m.renderMenu()
	if view == "" {
		t.Fatal("renderMenu returned empty string")
	}
}
