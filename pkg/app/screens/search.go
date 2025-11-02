package screens

import (
	"fmt"

	"github.com/charmbracelet/bubbles/textinput"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/kerbaras/mangas/pkg/app/styles"
	"github.com/kerbaras/mangas/pkg/data"
	"github.com/kerbaras/mangas/pkg/services"
	"github.com/kerbaras/mangas/pkg/sources"
)

type SearchScreen struct {
	source     sources.Source
	downloader *services.Downloader
	input      textinput.Model
	results    []data.Manga
	selected   int
	searching  bool
	width      int
	height     int
	err        error
}

func NewSearchScreen(source sources.Source, downloader *services.Downloader) *SearchScreen {
	ti := textinput.New()
	ti.Placeholder = "Search manga..."
	ti.Focus()
	ti.CharLimit = 100
	ti.Width = 50

	return &SearchScreen{
		source:     source,
		downloader: downloader,
		input:      ti,
		results:    []data.Manga{},
		selected:   0,
	}
}

func (s *SearchScreen) Init() tea.Cmd {
	return textinput.Blink
}

func (s *SearchScreen) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	var cmd tea.Cmd

	switch msg := msg.(type) {
	case tea.WindowSizeMsg:
		s.width = msg.Width
		s.height = msg.Height

	case tea.KeyMsg:
		// If searching, don't process keys
		if s.searching {
			return s, nil
		}

		switch msg.String() {
		case "enter":
			if s.input.Focused() {
				// Perform search
				query := s.input.Value()
				if query != "" {
					s.searching = true
					return s, s.performSearch(query)
				}
			} else if len(s.results) > 0 {
				// Start download for selected manga
				manga := s.results[s.selected]
				return s, s.startDownload(manga.ID)
			}

		case "esc":
			// Switch focus between input and results
			if s.input.Focused() {
				s.input.Blur()
			} else {
				s.input.Focus()
				cmd = textinput.Blink
			}

		case "up", "k":
			if !s.input.Focused() && len(s.results) > 0 {
				s.selected--
				if s.selected < 0 {
					s.selected = len(s.results) - 1
				}
			}

		case "down", "j":
			if !s.input.Focused() && len(s.results) > 0 {
				s.selected++
				if s.selected >= len(s.results) {
					s.selected = 0
				}
			}
		}

	case searchResultMsg:
		s.searching = false
		s.results = msg.results
		s.selected = 0
		s.err = msg.err
		if len(s.results) > 0 {
			s.input.Blur()
		}

	case downloadStartedMsg:
		if msg.err != nil {
			s.err = msg.err
		} else {
			// Switch to library view
			return s, func() tea.Msg {
				return SwitchScreenMsg{Screen: "library", Data: nil}
			}
		}
	}

	// Update text input
	if s.input.Focused() {
		s.input, cmd = s.input.Update(msg)
	}

	return s, cmd
}

func (s *SearchScreen) View() string {
	if s.width == 0 {
		return "Loading..."
	}

	header := styles.TitleStyle.Render("🔍 Search Manga")

	// Input field
	inputStyle := styles.InputStyle
	if s.input.Focused() {
		inputStyle = styles.FocusedInputStyle
	}
	inputView := inputStyle.Render(s.input.View())

	var errorMsg string
	if s.err != nil {
		errorMsg = styles.StatusError.Render(fmt.Sprintf("Error: %s", s.err))
		errorMsg += "\n\n"
	}

	var resultsView string
	if s.searching {
		resultsView = styles.StatusDownloading.Render("Searching...")
	} else if len(s.results) > 0 {
		resultsView = s.renderResults()
	} else if s.input.Value() != "" && !s.searching {
		resultsView = styles.MutedStyle.Render("No results found")
	}

	help := styles.HelpStyle.Render(
		"enter: search/download • esc: switch focus • ↑/k ↓/j: navigate • tab: switch view • q: quit",
	)

	content := fmt.Sprintf("%s\n\n%s\n\n%s%s\n\n%s",
		header,
		inputView,
		errorMsg,
		resultsView,
		help,
	)

	return content
}

func (s *SearchScreen) renderResults() string {
	var result string
	result += styles.SubtitleStyle.Render(fmt.Sprintf("Found %d results:", len(s.results)))
	result += "\n\n"

	for i, manga := range s.results {
		cardStyle := styles.CardStyle
		if i == s.selected && !s.input.Focused() {
			cardStyle = styles.ActiveCardStyle
		}

		title := styles.TitleStyle.Render(manga.Name)

		desc := manga.Description
		if len(desc) > 120 {
			desc = desc[:117] + "..."
		}
		description := styles.TextStyle.Render(desc)

		source := styles.MutedStyle.Render(fmt.Sprintf("Source: %s • ID: %s", manga.Source, manga.ID))

		cardContent := lipgloss.JoinVertical(
			lipgloss.Left,
			title,
			description,
			source,
		)

		card := cardStyle.Width(s.width - 6).Render(cardContent)
		result += card + "\n"
	}

	return result
}

// Messages
type searchResultMsg struct {
	results []data.Manga
	err     error
}

type downloadStartedMsg struct {
	err error
}

// Define shared message for screen switching
type SwitchScreenMsg struct {
	Screen string
	Data   interface{}
}

// Commands
func (s *SearchScreen) performSearch(query string) tea.Cmd {
	return func() tea.Msg {
		results, err := s.source.Search(query)
		// Convert []*data.Manga to []data.Manga for compatibility
		var mangaList []data.Manga
		for _, m := range results {
			if m != nil {
				mangaList = append(mangaList, *m)
			}
		}
		return searchResultMsg{results: mangaList, err: err}
	}
}

func (s *SearchScreen) startDownload(mangaID string) tea.Cmd {
	return func() tea.Msg {
		// Get manga from repository or source
		manga, err := s.source.GetManga(mangaID)
		if err != nil {
			return downloadStartedMsg{err: err}
		}
		
		// Get chapters from source
		chapters, err := s.source.GetChapters(manga)
		if err != nil {
			return downloadStartedMsg{err: err}
		}
		
		// Start download in background
		go s.downloader.DownloadManga(manga, chapters)
		return downloadStartedMsg{err: nil}
	}
}
