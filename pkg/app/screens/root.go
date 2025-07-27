package screens

import (
	tea "github.com/charmbracelet/bubbletea"
)

type RootScreen struct {
	width  int
	height int
}

func NewRootScreen() *RootScreen {
	return &RootScreen{}
}

func (s *RootScreen) Init() tea.Cmd {
	return tea.Batch(tea.Println("Hello, World!"), tea.EnterAltScreen)
}

func (m *RootScreen) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	var cmds []tea.Cmd

	// Handle global messages
	switch msg := msg.(type) {
	case tea.KeyMsg:
		switch msg.String() {
		case "ctrl+c", "q":
			return m, tea.Quit
		}

	case tea.WindowSizeMsg:
		m.width = msg.Width
		m.height = msg.Height
	}

	return m, tea.Batch(cmds...)
}

func (s *RootScreen) View() string {
	return "Hello, World!"
}
