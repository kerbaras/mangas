package services

import (
	"fmt"
	"os"
	"path/filepath"
	"strconv"
	"strings"

	"github.com/kerbaras/mangas/pkg/data"
	"github.com/kerbaras/mangas/pkg/sources"
)

// MangaController orchestrates interactions between sources, repositories, and downloaders
// It provides a clean API for both CLI and TUI to use without duplicating logic
type MangaController struct {
	source      sources.Source
	repo        Repository
	downloader  *Downloader
	downloadDir string
}

// ControllerConfig holds configuration for creating a controller
type ControllerConfig struct {
	SourceType  string // "mangadex", etc.
	DownloadDir string // If empty, uses ~/.mangas/downloads
}

// NewMangaController creates a new controller with default configuration
func NewMangaController() *MangaController {
	return NewMangaControllerWithConfig(ControllerConfig{
		SourceType: "mangadex",
	})
}

// NewMangaControllerWithConfig creates a controller with custom configuration
func NewMangaControllerWithConfig(config ControllerConfig) *MangaController {
	// Initialize source based on type
	var source sources.Source
	switch config.SourceType {
	case "mangadex", "":
		source = sources.NewMangaDex()
	default:
		source = sources.NewMangaDex() // Default fallback
	}

	// Initialize repository
	repo := data.NewDuckDBRepository()

	// Determine download directory
	downloadDir := config.DownloadDir
	if downloadDir == "" {
		homeDir, _ := os.UserHomeDir()
		downloadDir = filepath.Join(homeDir, ".mangas", "downloads")
	}

	// Ensure download directory exists
	os.MkdirAll(downloadDir, 0755)

	// Initialize downloader
	downloader := NewDownloader(source, repo, downloadDir)

	return &MangaController{
		source:      source,
		repo:        repo,
		downloader:  downloader,
		downloadDir: downloadDir,
	}
}

// SearchManga searches for manga by query string
func (c *MangaController) SearchManga(query string) ([]*data.Manga, error) {
	if query == "" {
		return nil, fmt.Errorf("search query cannot be empty")
	}
	return c.source.Search(query)
}

// GetManga retrieves a manga by ID from source
func (c *MangaController) GetManga(mangaID string) (*data.Manga, error) {
	if mangaID == "" {
		return nil, fmt.Errorf("manga ID cannot be empty")
	}
	return c.source.GetManga(mangaID)
}

// GetMangaFromLibrary retrieves a manga from the local library
func (c *MangaController) GetMangaFromLibrary(mangaID string) (*data.Manga, error) {
	if mangaID == "" {
		return nil, fmt.Errorf("manga ID cannot be empty")
	}
	return c.repo.GetManga(mangaID)
}

// FindMangaByName searches for a manga in the library by name (case-insensitive)
func (c *MangaController) FindMangaByName(name string) (*data.Manga, error) {
	if name == "" {
		return nil, fmt.Errorf("manga name cannot be empty")
	}

	mangas, err := c.repo.ListMangas()
	if err != nil {
		return nil, fmt.Errorf("failed to list mangas: %w", err)
	}

	for _, m := range mangas {
		if strings.EqualFold(m.Name, name) {
			return m, nil
		}
	}

	return nil, fmt.Errorf("manga not found in library: %s", name)
}

// GetChapters retrieves chapters for a manga from source
func (c *MangaController) GetChapters(manga *data.Manga) ([]*data.Chapter, error) {
	if manga == nil {
		return nil, fmt.Errorf("manga cannot be nil")
	}
	return c.source.GetChapters(manga)
}

// GetChaptersFromLibrary retrieves chapters for a manga from the local library
func (c *MangaController) GetChaptersFromLibrary(mangaID string) ([]*data.Chapter, error) {
	if mangaID == "" {
		return nil, fmt.Errorf("manga ID cannot be empty")
	}
	return c.repo.GetChapters(mangaID)
}

// AddMangaToLibrary adds a manga to the library with its chapters metadata
func (c *MangaController) AddMangaToLibrary(manga *data.Manga) error {
	if manga == nil {
		return fmt.Errorf("manga cannot be nil")
	}

	// Save manga
	if err := c.repo.SaveManga(manga); err != nil {
		return fmt.Errorf("failed to save manga: %w", err)
	}

	// Get and save chapters
	chapters, err := c.source.GetChapters(manga)
	if err != nil {
		return fmt.Errorf("failed to get chapters: %w", err)
	}

	for _, chapter := range chapters {
		chapter.MangaID = manga.ID
		if err := c.repo.SaveChapter(chapter); err != nil {
			// Log but don't fail on individual chapter errors
			continue
		}
	}

	return nil
}

// ListLibraryMangas lists all mangas in the library
func (c *MangaController) ListLibraryMangas() ([]*data.Manga, error) {
	return c.repo.ListMangas()
}

// DeleteMangaFromLibrary removes a manga and its chapters from the library
func (c *MangaController) DeleteMangaFromLibrary(mangaID string) error {
	if mangaID == "" {
		return fmt.Errorf("manga ID cannot be empty")
	}
	return c.repo.DeleteManga(mangaID)
}

// DownloadOptions specifies options for downloading manga chapters
type DownloadOptions struct {
	Language      string   // Language code (e.g., "en", "ja")
	ChapterRange  string   // Chapter range (e.g., "1-10")
	ChapterIDs    []string // Specific chapter IDs to download
	ProgressChan  chan<- DownloadProgress // Optional progress channel
}

// DownloadManga downloads manga chapters with the specified options
func (c *MangaController) DownloadManga(manga *data.Manga, options DownloadOptions) error {
	if manga == nil {
		return fmt.Errorf("manga cannot be nil")
	}

	// Get all chapters
	chapters, err := c.source.GetChapters(manga)
	if err != nil {
		return fmt.Errorf("failed to get chapters: %w", err)
	}

	// Filter chapters based on options
	filteredChapters := c.filterChapters(chapters, options)

	if len(filteredChapters) == 0 {
		return fmt.Errorf("no chapters to download after applying filters")
	}

	// Start download
	return c.downloader.DownloadManga(manga, filteredChapters)
}

// DownloadChapter downloads a single chapter
func (c *MangaController) DownloadChapter(manga *data.Manga, chapter *data.Chapter) error {
	if manga == nil {
		return fmt.Errorf("manga cannot be nil")
	}
	if chapter == nil {
		return fmt.Errorf("chapter cannot be nil")
	}
	return c.downloader.DownloadChapter(manga, chapter)
}

// GetProgressChannel returns the channel for receiving download progress updates
func (c *MangaController) GetProgressChannel() <-chan DownloadProgress {
	return c.downloader.GetProgressChannel()
}

// GetDownloadDirectory returns the configured download directory
func (c *MangaController) GetDownloadDirectory() string {
	return c.downloadDir
}

// Close cleans up controller resources
func (c *MangaController) Close() error {
	c.downloader.Close()
	return nil
}

// Helper methods

// filterChapters filters chapters based on download options
func (c *MangaController) filterChapters(chapters []*data.Chapter, options DownloadOptions) []*data.Chapter {
	var filtered []*data.Chapter

	// Filter by language
	if options.Language != "" {
		for _, ch := range chapters {
			if ch.Language == options.Language {
				filtered = append(filtered, ch)
			}
		}
	} else {
		filtered = chapters
	}

	// Filter by specific chapter IDs
	if len(options.ChapterIDs) > 0 {
		idMap := make(map[string]bool)
		for _, id := range options.ChapterIDs {
			idMap[id] = true
		}

		var idFiltered []*data.Chapter
		for _, ch := range filtered {
			if idMap[ch.ID] {
				idFiltered = append(idFiltered, ch)
			}
		}
		filtered = idFiltered
	}

	// Filter by chapter range
	if options.ChapterRange != "" {
		filtered = c.filterByRange(filtered, options.ChapterRange)
	}

	return filtered
}

// filterByRange filters chapters by a range string (e.g., "1-10")
func (c *MangaController) filterByRange(chapters []*data.Chapter, rangeStr string) []*data.Chapter {
	parts := strings.Split(rangeStr, "-")
	if len(parts) != 2 {
		return chapters // Invalid range, return all
	}

	start, err1 := strconv.ParseFloat(parts[0], 64)
	end, err2 := strconv.ParseFloat(parts[1], 64)
	if err1 != nil || err2 != nil {
		return chapters // Invalid range, return all
	}

	var filtered []*data.Chapter
	for _, ch := range chapters {
		chNum, err := strconv.ParseFloat(ch.Number, 64)
		if err != nil {
			continue
		}
		if chNum >= start && chNum <= end {
			filtered = append(filtered, ch)
		}
	}

	return filtered
}

// UpdateChapterStatus updates the download status of a chapter
func (c *MangaController) UpdateChapterStatus(chapterID string, downloaded bool, filePath string) error {
	if chapterID == "" {
		return fmt.Errorf("chapter ID cannot be empty")
	}
	return c.repo.UpdateChapterStatus(chapterID, downloaded, filePath)
}

// SaveManga saves a manga to the repository
func (c *MangaController) SaveManga(manga *data.Manga) error {
	if manga == nil {
		return fmt.Errorf("manga cannot be nil")
	}
	return c.repo.SaveManga(manga)
}

// SaveChapter saves a chapter to the repository
func (c *MangaController) SaveChapter(chapter *data.Chapter) error {
	if chapter == nil {
		return fmt.Errorf("chapter cannot be nil")
	}
	return c.repo.SaveChapter(chapter)
}
