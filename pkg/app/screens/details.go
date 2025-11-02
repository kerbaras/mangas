package screens

import (
	"fmt"
	"strings"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/kerbaras/mangas/pkg/app/components"
	"github.com/kerbaras/mangas/pkg/app/styles"
	"github.com/kerbaras/mangas/pkg/data"
	"github.com/kerbaras/mangas/pkg/services"
)

type DetailsScreen struct {
	repo           *data.Repository
	downloader     *services.Downloader
	mangaID        string
	manga          *data.Manga
	chapters       []*data.Chapter
	selectedChapter int
	progressTracker *components.ProgressTracker
	width          int
	height         int
	err            error
}

func NewDetailsScreen(repo *data.Repository, downloader *services.Downloader, mangaID string) *DetailsScreen {
	return &DetailsScreen{
		repo:            repo,
		downloader:      downloader,
		mangaID:         mangaID,
		progressTracker: components.NewProgressTracker(80),
	}
}

func (s *DetailsScreen) Init() tea.Cmd {
	return tea.Batch(
		s.loadDetails,
		s.listenForProgress,
	)
}

func (s *DetailsScreen) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	switch msg := msg.(type) {
	case tea.WindowSizeMsg:
		s.width = msg.Width
		s.height = msg.Height
		s.progressTracker = components.NewProgressTracker(msg.Width - 4)

	case tea.KeyMsg:
		switch msg.String() {
		case "up", "k":
			if s.selectedChapter > 0 {
				s.selectedChapter--
			}
		case "down", "j":
			if s.selectedChapter < len(s.chapters)-1 {
				s.selectedChapter++
			}
		case "r":
			return s, s.loadDetails
		case "e":
			// Generate EPUB
			return s, s.generateEPUB()
		case "esc", "backspace":
			// Go back to library
			return s, func() tea.Msg {
				return SwitchScreenMsg{Screen: "library", Data: nil}
			}
		}

	case detailsLoadedMsg:
		s.manga = msg.manga
		s.chapters = msg.chapters
		s.err = msg.err

	case services.DownloadProgress:
		s.progressTracker.Update(msg)
		return s, s.listenForProgress

	case epubGeneratedMsg:
		if msg.err != nil {
			s.err = msg.err
		}
		return s, s.loadDetails
	}

	return s, nil
}

func (s *DetailsScreen) View() string {
	if s.width == 0 || s.manga == nil {
		return "Loading..."
	}

	header := styles.TitleStyle.Render(fmt.Sprintf("📖 %s", s.manga.Name))

	var errorMsg string
	if s.err != nil {
		errorMsg = styles.StatusError.Render(fmt.Sprintf("Error: %s", s.err))
		errorMsg += "\n\n"
	}

	// Manga info section
	info := s.renderMangaInfo()

	// Chapters list
	chaptersList := s.renderChaptersList()

	// Progress section
	progressView := s.progressTracker.View()

	help := styles.HelpStyle.Render(
		"↑/k ↓/j: navigate • e: generate EPUB • r: refresh • esc: back • q: quit",
	)

	content := fmt.Sprintf("%s\n\n%s%s\n%s\n%s\n%s",
		header,
		errorMsg,
		info,
		chaptersList,
		progressView,
		help,
	)

	return content
}

func (s *DetailsScreen) renderMangaInfo() string {
	status := styles.StatusStyle(s.manga.Status).Render(s.manga.Status)
	if s.manga.Status == "" {
		status = styles.MutedStyle.Render("Ready")
	}

	desc := s.manga.Description
	if len(desc) > 200 {
		desc = desc[:197] + "..."
	}

	info := lipgloss.JoinVertical(
		lipgloss.Left,
		styles.TextStyle.Render(desc),
		"",
		styles.MutedStyle.Render(fmt.Sprintf("Source: %s", s.manga.Source)),
		status,
		"",
	)

	return styles.CardStyle.Width(s.width - 4).Render(info)
}

func (s *DetailsScreen) renderChaptersList() string {
	if len(s.chapters) == 0 {
		return styles.MutedStyle.Render("No chapters available")
	}

	var b strings.Builder
	b.WriteString(styles.SubtitleStyle.Render(fmt.Sprintf("Chapters (%d total):", len(s.chapters))))
	b.WriteString("\n\n")

	// Show limited chapters (scrollable view would be better, but simplified for now)
	start := 0
	end := len(s.chapters)
	if end > 10 {
		// Show 10 chapters around selected
		start = s.selectedChapter - 5
		if start < 0 {
			start = 0
		}
		end = start + 10
		if end > len(s.chapters) {
			end = len(s.chapters)
			start = end - 10
			if start < 0 {
				start = 0
			}
		}
	}

	for i := start; i < end; i++ {
		ch := s.chapters[i]
		chapterText := fmt.Sprintf("Ch. %s", ch.Number)
		if ch.Volume != "" && ch.Volume != "0" {
			chapterText = fmt.Sprintf("Vol. %s, %s", ch.Volume, chapterText)
		}
		if ch.Title != "" {
			chapterText = fmt.Sprintf("%s: %s", chapterText, ch.Title)
		}

		statusIcon := "○"
		statusColor := styles.MutedStyle
		if ch.Downloaded {
			statusIcon = "●"
			statusColor = styles.StatusCompleted
		}

		line := fmt.Sprintf("%s %s", statusIcon, chapterText)
		
		if i == s.selectedChapter {
			line = styles.SelectedStyle.Render(line)
		} else {
			line = statusColor.Render(line)
		}

		b.WriteString(line)
		b.WriteString("\n")
	}

	if len(s.chapters) > 10 {
		b.WriteString("\n")
		b.WriteString(styles.MutedStyle.Render(
			fmt.Sprintf("Showing %d-%d of %d chapters", start+1, end, len(s.chapters)),
		))
	}

	return b.String()
}

// Messages
type detailsLoadedMsg struct {
	manga    *data.Manga
	chapters []*data.Chapter
	err      error
}

// Commands
func (s *DetailsScreen) loadDetails() tea.Msg {
	manga, err := s.repo.GetManga(s.mangaID)
	if err != nil {
		return detailsLoadedMsg{err: err}
	}
	if manga == nil {
		return detailsLoadedMsg{err: fmt.Errorf("manga not found")}
	}

	chapters, err := s.repo.GetChapters(s.mangaID)
	if err != nil {
		return detailsLoadedMsg{manga: manga, err: err}
	}

	return detailsLoadedMsg{manga: manga, chapters: chapters}
}

func (s *DetailsScreen) generateEPUB() tea.Cmd {
	return func() tea.Msg {
		// Note: With the new streaming architecture, EPUBs are created during download
		// This function is now a no-op or could trigger a re-download if needed
		// For now, we'll return an error indicating EPUBs are created during download
		return epubGeneratedMsg{
			path: "",
			err:  fmt.Errorf("EPUBs are now created automatically during chapter download"),
		}
	}
}

func (s *DetailsScreen) listenForProgress() tea.Msg {
	return <-s.downloader.GetProgressChannel()
}
