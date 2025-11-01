package components

import (
	"fmt"
	"strings"

	"github.com/kerbaras/mangas/pkg/app/styles"
	"github.com/kerbaras/mangas/pkg/services"
)

type ProgressTracker struct {
	downloads map[string]*services.DownloadProgress
	width     int
}

func NewProgressTracker(width int) *ProgressTracker {
	return &ProgressTracker{
		downloads: make(map[string]*services.DownloadProgress),
		width:     width,
	}
}

func (p *ProgressTracker) Update(progress services.DownloadProgress) {
	key := progress.MangaID + ":" + progress.ChapterID
	if progress.Status == "complete" && progress.ChapterID != "" {
		// Remove completed chapter downloads
		delete(p.downloads, key)
	} else {
		prog := progress // Copy
		p.downloads[key] = &prog
	}
}

func (p *ProgressTracker) Clear() {
	p.downloads = make(map[string]*services.DownloadProgress)
}

func (p *ProgressTracker) HasActive() bool {
	return len(p.downloads) > 0
}

func (p *ProgressTracker) View() string {
	if len(p.downloads) == 0 {
		return ""
	}

	var b strings.Builder
	b.WriteString(styles.TitleStyle.Render("Active Downloads"))
	b.WriteString("\n\n")

	for _, progress := range p.downloads {
		// Chapter info
		chapterText := fmt.Sprintf("Chapter %s", progress.ChapterNumber)
		if progress.ChapterNumber == "" {
			chapterText = "Processing manga"
		}

		b.WriteString(styles.TextStyle.Render(chapterText))
		b.WriteString("\n")

		// Status and progress
		statusText := progress.Status
		if progress.TotalPages > 0 {
			percentage := float64(progress.CurrentPage) / float64(progress.TotalPages) * 100
			statusText = fmt.Sprintf("%s (%d/%d pages - %.0f%%)",
				progress.Status, progress.CurrentPage, progress.TotalPages, percentage)

			// Progress bar
			bar := renderProgressBar(progress.CurrentPage, progress.TotalPages, p.width-4)
			b.WriteString(bar)
			b.WriteString("\n")
		}

		statusStyle := styles.StatusStyle(progress.Status)
		b.WriteString(statusStyle.Render(statusText))
		b.WriteString("\n")

		if progress.Error != nil {
			errMsg := styles.StatusError.Render(fmt.Sprintf("Error: %s", progress.Error))
			b.WriteString(errMsg)
			b.WriteString("\n")
		}

		b.WriteString("\n")
	}

	return b.String()
}

func renderProgressBar(current, total, width int) string {
	if total == 0 {
		return ""
	}

	filled := int(float64(current) / float64(total) * float64(width))
	if filled > width {
		filled = width
	}

	bar := strings.Repeat("█", filled) + strings.Repeat("░", width-filled)
	return styles.ProgressBarStyle.Render(bar)
}

// SimpleProgress renders a simple progress bar
func SimpleProgress(current, total, width int) string {
	return renderProgressBar(current, total, width)
}
