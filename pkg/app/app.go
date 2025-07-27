package app

import (
	tea "github.com/charmbracelet/bubbletea"
	"github.com/kerbaras/mangas/pkg/app/screens"
)

type App struct {
}

func NewApp() *App {
	return &App{}
}

func (a *App) Run() error {
	model := screens.NewRootScreen()
	p := tea.NewProgram(model, tea.WithAltScreen(), tea.WithMouseCellMotion())
	_, err := p.Run()
	return err
}
