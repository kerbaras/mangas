package screens

import (
	"fmt"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/kerbaras/mangas/pkg/app/components"
	"github.com/kerbaras/mangas/pkg/app/styles"
	"github.com/kerbaras/mangas/pkg/data"
	"github.com/kerbaras/mangas/pkg/services"
)

type LibraryScreen struct {
	repo         *data.Repository
	downloader   *services.Downloader
	mangaList    *components.MangaList
	width        int
	height       int
	err          error
}

func NewLibraryScreen(repo *data.Repository, downloader *services.Downloader) *LibraryScreen {
	return &LibraryScreen{
		repo:       repo,
		downloader: downloader,
		mangaList:  components.NewMangaList(),
	}
}

func (s *LibraryScreen) Init() tea.Cmd {
	return s.loadLibrary
}

func (s *LibraryScreen) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	switch msg := msg.(type) {
	case tea.WindowSizeMsg:
		s.width = msg.Width
		s.height = msg.Height
		s.mangaList.Width = msg.Width - 4
		s.mangaList.Height = msg.Height - 10
		
	case tea.KeyMsg:
		switch msg.String() {
		case "up", "k":
			s.mangaList.Prev()
		case "down", "j":
			s.mangaList.Next()
		case "r":
			return s, s.loadLibrary
		case "d":
			// Delete selected manga
			selected := s.mangaList.Selected()
			if selected != nil {
				return s, s.deleteManga(selected.Manga.ID)
			}
		case "e":
			// Generate EPUB for selected manga
			selected := s.mangaList.Selected()
			if selected != nil {
				return s, s.generateEPUB(selected.Manga.ID)
			}
		case "enter":
			// Return selected manga to switch to details view
			selected := s.mangaList.Selected()
			if selected != nil {
				return s, func() tea.Msg {
					return SwitchScreenMsg{Screen: "details", Data: selected.Manga.ID}
				}
			}
		}
		
	case libraryLoadedMsg:
		s.mangaList.SetItems(msg.items)
		s.err = msg.err
		
	case epubGeneratedMsg:
		if msg.err != nil {
			s.err = msg.err
		}
		return s, s.loadLibrary
		
	case mangaDeletedMsg:
		if msg.err != nil {
			s.err = msg.err
		}
		return s, s.loadLibrary
	}
	
	return s, nil
}

func (s *LibraryScreen) View() string {
	if s.width == 0 {
		return "Loading..."
	}

	header := styles.TitleStyle.Render("📚 Manga Library")
	
	var errorMsg string
	if s.err != nil {
		errorMsg = styles.StatusError.Render(fmt.Sprintf("Error: %s", s.err))
		errorMsg += "\n\n"
	}
	
	listView := s.mangaList.View()
	
	help := styles.HelpStyle.Render(
		"↑/k: up • ↓/j: down • enter: details • e: generate EPUB • d: delete • r: refresh • tab: switch view • q: quit",
	)
	
	content := fmt.Sprintf("%s\n\n%s%s\n%s", header, errorMsg, listView, help)
	
	return content
}

// Messages
type libraryLoadedMsg struct {
	items []components.MangaListItem
	err   error
}

type epubGeneratedMsg struct {
	path string
	err  error
}

type mangaDeletedMsg struct {
	err error
}

// Commands
func (s *LibraryScreen) loadLibrary() tea.Msg {
	mangas, err := s.repo.ListMangas()
	if err != nil {
		return libraryLoadedMsg{err: err}
	}
	
	items := make([]components.MangaListItem, len(mangas))
	for i, manga := range mangas {
		_, total, downloaded, _ := s.repo.GetMangaWithChapterCount(manga.ID)
		items[i] = components.MangaListItem{
			Manga:           manga,
			ChapterCount:    total,
			DownloadedCount: downloaded,
		}
	}
	
	return libraryLoadedMsg{items: items}
}

func (s *LibraryScreen) generateEPUB(mangaID string) tea.Cmd {
	return func() tea.Msg {
		// Note: With the new streaming architecture, EPUBs are created during download
		// This function is now a no-op or could trigger a re-download if needed
		return epubGeneratedMsg{
			path: "",
			err:  fmt.Errorf("EPUBs are now created automatically during chapter download"),
		}
	}
}

func (s *LibraryScreen) deleteManga(mangaID string) tea.Cmd {
	return func() tea.Msg {
		err := s.repo.DeleteManga(mangaID)
		return mangaDeletedMsg{err: err}
	}
}
