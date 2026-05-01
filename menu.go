package main

import (
	"fmt"
	"io"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/ssh"
)

type MenuModel struct {
	session  ssh.Session
	clientID string
	choices  []string
	cursor   int
	choice   string
}

func RunMenu(s ssh.Session, input io.Reader, clientID string) (string, error) {
	m := MenuModel{
		session:  s,
		clientID: clientID,
		choices:  []string{"Resume", "Disconnect"},
	}
	p := tea.NewProgram(m, tea.WithInput(input), tea.WithOutput(s), tea.WithAltScreen())
	finalModel, err := p.Run()
	if err != nil {
		return "", err
	}
	if finalM, ok := finalModel.(MenuModel); ok {
		return finalM.choice, nil
	}
	return "", nil
}

func (m MenuModel) Init() tea.Cmd {
	return nil
}

func (m MenuModel) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	switch msg := msg.(type) {
	case tea.KeyMsg:
		switch msg.String() {
		case "ctrl+c", "q", "esc":
			m.choice = "Resume"
			return m, tea.Quit
		case "up", "k":
			if m.cursor > 0 {
				m.cursor--
			}
		case "down", "j":
			if m.cursor < len(m.choices)-1 {
				m.cursor++
			}
		case "enter", " ":
			m.choice = m.choices[m.cursor]
			return m, tea.Quit
		}
	}
	return m, nil
}

func (m MenuModel) View() string {
	s := fmt.Sprintf("\n\n🌈 Prism Pause Menu 🌈\n👤 Client: %s\n\n", m.clientID)

	for i, choice := range m.choices {
		cursor := " "
		if m.cursor == i {
			cursor = ">"
		}
		s += fmt.Sprintf("%s %s\n", cursor, choice)
	}

	s += "\nPress j/k to move, enter to select, esc to return to session.\n"
	return s
}
