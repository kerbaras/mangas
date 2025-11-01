package components

import (
	"fmt"
	"strings"

	"github.com/charmbracelet/lipgloss"
	"github.com/kerbaras/mangas/pkg/app/styles"
	"github.com/kerbaras/mangas/pkg/data"
)

type MangaListItem struct {
	Manga            *data.Manga
	ChapterCount     int
	DownloadedCount  int
}

type MangaList struct {
	Items         []MangaListItem
	SelectedIndex int
	Width         int
	Height        int
}

func NewMangaList() *MangaList {
	return &MangaList{
		Items:         []MangaListItem{},
		SelectedIndex: 0,
		Width:         80,
		Height:        20,
	}
}

func (m *MangaList) SetItems(items []MangaListItem) {
	m.Items = items
	if m.SelectedIndex >= len(items) && len(items) > 0 {
		m.SelectedIndex = len(items) - 1
	}
	if len(items) == 0 {
		m.SelectedIndex = 0
	}
}

func (m *MangaList) Next() {
	if len(m.Items) == 0 {
		return
	}
	m.SelectedIndex++
	if m.SelectedIndex >= len(m.Items) {
		m.SelectedIndex = 0
	}
}

func (m *MangaList) Prev() {
	if len(m.Items) == 0 {
		return
	}
	m.SelectedIndex--
	if m.SelectedIndex < 0 {
		m.SelectedIndex = len(m.Items) - 1
	}
}

func (m *MangaList) Selected() *MangaListItem {
	if len(m.Items) == 0 || m.SelectedIndex >= len(m.Items) {
		return nil
	}
	return &m.Items[m.SelectedIndex]
}

func (m *MangaList) View() string {
	if len(m.Items) == 0 {
		emptyMsg := styles.MutedStyle.Render("No manga in library")
		return lipgloss.Place(m.Width, m.Height, lipgloss.Center, lipgloss.Center, emptyMsg)
	}

	var b strings.Builder
	
	for i, item := range m.Items {
		cardStyle := styles.CardStyle
		if i == m.SelectedIndex {
			cardStyle = styles.ActiveCardStyle
		}

		// Build card content
		title := styles.TitleStyle.Render(item.Manga.Name)
		
		statusText := fmt.Sprintf("Status: %s", item.Manga.Status)
		if item.Manga.Status == "" {
			statusText = "Status: Ready"
		}
		status := styles.StatusStyle(item.Manga.Status).Render(statusText)
		
		chapterInfo := styles.MutedStyle.Render(
			fmt.Sprintf("Chapters: %d / %d downloaded", item.DownloadedCount, item.ChapterCount),
		)
		
		source := styles.MutedStyle.Render(fmt.Sprintf("Source: %s", item.Manga.Source))
		
		// Truncate description
		desc := item.Manga.Description
		if len(desc) > 80 {
			desc = desc[:77] + "..."
		}
		description := styles.TextStyle.Render(desc)
		
		cardContent := lipgloss.JoinVertical(
			lipgloss.Left,
			title,
			description,
			"",
			chapterInfo,
			status,
			source,
		)
		
		card := cardStyle.Width(m.Width - 4).Render(cardContent)
		b.WriteString(card)
		b.WriteString("\n")
	}

	return b.String()
}
