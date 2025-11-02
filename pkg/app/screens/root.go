package screens

import (
	"fmt"
	"os"
	"path/filepath"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/kerbaras/mangas/pkg/app/styles"
	"github.com/kerbaras/mangas/pkg/data"
	"github.com/kerbaras/mangas/pkg/services"
	"github.com/kerbaras/mangas/pkg/sources"
)

type screenType int

const (
	libraryView screenType = iota
	searchView
	detailsView
)

type RootScreen struct {
	repo       *data.Repository
	source     sources.Source
	downloader *services.Downloader

	currentView screenType
	library     *LibraryScreen
	search      *SearchScreen
	details     *DetailsScreen

	width  int
	height int
}

func NewRootScreen() *RootScreen {
	// Initialize dependencies
	repo := data.NewDuckDBRepository()
	source := sources.NewMangaDex()
	
	homeDir, _ := os.UserHomeDir()
	downloadDir := filepath.Join(homeDir, ".mangas", "downloads")
	
	downloader := services.NewDownloader(source, repo, downloadDir)

	// Create screens
	library := NewLibraryScreen(repo, downloader)
	search := NewSearchScreen(source, downloader)

	return &RootScreen{
		repo:        repo,
		source:      source,
		downloader:  downloader,
		currentView: libraryView,
		library:     library,
		search:      search,
	}
}

func (r *RootScreen) Init() tea.Cmd {
	return r.library.Init()
}

func (r *RootScreen) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	var cmd tea.Cmd

	switch msg := msg.(type) {
	case tea.WindowSizeMsg:
		r.width = msg.Width
		r.height = msg.Height

	case tea.KeyMsg:
		switch msg.String() {
		case "q", "ctrl+c":
			return r, tea.Quit
		case "tab":
			// Cycle through views
			if r.currentView == detailsView {
				// Can't tab away from details, use esc
				break
			}
			r.currentView = (r.currentView + 1) % 2
			if r.currentView == searchView {
				cmd = r.search.Init()
			} else {
				cmd = r.library.Init()
			}
			return r, cmd
		}

	case SwitchScreenMsg:
		// Handle screen switching from sub-screens
		switch msg.Screen {
		case "library":
			r.currentView = libraryView
			cmd = r.library.Init()
		case "search":
			r.currentView = searchView
			cmd = r.search.Init()
		case "details":
			if mangaID, ok := msg.Data.(string); ok {
				r.details = NewDetailsScreen(r.repo, r.downloader, mangaID)
				r.currentView = detailsView
				cmd = r.details.Init()
			}
		}
		return r, cmd
	}

	// Forward message to active screen
	switch r.currentView {
	case libraryView:
		newModel, newCmd := r.library.Update(msg)
		r.library = newModel.(*LibraryScreen)
		return r, newCmd
	case searchView:
		newModel, newCmd := r.search.Update(msg)
		r.search = newModel.(*SearchScreen)
		return r, newCmd
	case detailsView:
		if r.details != nil {
			newModel, newCmd := r.details.Update(msg)
			r.details = newModel.(*DetailsScreen)
			return r, newCmd
		}
	}

	return r, cmd
}

func (r *RootScreen) View() string {
	// Render tabs
	tabs := r.renderTabs()

	// Render active screen
	var content string
	switch r.currentView {
	case libraryView:
		content = r.library.View()
	case searchView:
		content = r.search.View()
	case detailsView:
		if r.details != nil {
			content = r.details.View()
		}
	}

	return fmt.Sprintf("%s\n\n%s", tabs, content)
}

func (r *RootScreen) renderTabs() string {
	if r.currentView == detailsView {
		// Don't show tabs in details view
		return ""
	}

	libraryTab := "Library"
	searchTab := "Search"

	if r.currentView == libraryView {
		libraryTab = styles.ActiveTabStyle.Render(libraryTab)
		searchTab = styles.InactiveTabStyle.Render(searchTab)
	} else {
		libraryTab = styles.InactiveTabStyle.Render(libraryTab)
		searchTab = styles.ActiveTabStyle.Render(searchTab)
	}

	tabs := lipgloss.JoinHorizontal(lipgloss.Top, libraryTab, searchTab)
	return tabs
}
