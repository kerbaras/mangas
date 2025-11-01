package services

import (
	"fmt"
	"io"
	"net/http"
	"os"
	"path/filepath"
	"sync"
	"time"

	"github.com/kerbaras/mangas/pkg/data"
	"github.com/kerbaras/mangas/pkg/integrations"
	"github.com/kerbaras/mangas/pkg/sources"
)

// DownloadProgress represents the progress of a download operation
type DownloadProgress struct {
	MangaID       string
	ChapterID     string
	CurrentPage   int
	TotalPages    int
	Status        string // "downloading", "processing", "complete", "error"
	Error         error
	ChapterNumber string
}

// Repository interface needed by downloader
type Repository interface {
	SaveManga(manga *data.Manga) error
	GetManga(id string) (*data.Manga, error)
	GetChapters(mangaID string) ([]*data.Chapter, error)
	SaveChapter(chapter *data.Chapter) error
	UpdateChapterStatus(chapterID string, downloaded bool, filePath string) error
}

// Downloader orchestrates manga downloads from sources
type Downloader struct {
	source       sources.Source
	repo         Repository
	epubBuilder  *integrations.EPubBuilder
	downloadDir  string
	client       *http.Client
	rateLimiter  *time.Ticker
	progressChan chan DownloadProgress
}

func NewDownloader(source sources.Source, repo Repository, downloadDir string) *Downloader {
	epubBuilder := integrations.NewEPubBuilder()
	return &Downloader{
		source:       source,
		repo:         repo,
		epubBuilder:  epubBuilder,
		downloadDir:  downloadDir,
		client:       http.DefaultClient,
		rateLimiter:  time.NewTicker(500 * time.Millisecond), // 2 req/sec
		progressChan: make(chan DownloadProgress, 100),
	}
}

// GetProgressChannel returns the channel for receiving download progress updates
func (d *Downloader) GetProgressChannel() <-chan DownloadProgress {
	return d.progressChan
}

// DownloadManga downloads all chapters of a manga
func (d *Downloader) DownloadManga(manga *data.Manga, chapters []*data.Chapter) error {
	// Save manga to database
	manga.Status = "downloading"
	if err := d.repo.SaveManga(manga); err != nil {
		return fmt.Errorf("failed to save manga: %w", err)
	}

	if len(chapters) == 0 {
		chapters, _ = d.source.GetChapters(manga)
	}

	// Download chapters with concurrency control
	var wg sync.WaitGroup
	semaphore := make(chan struct{}, 3) // Max 3 concurrent downloads
	errorChan := make(chan error, len(chapters))

	for _, chapter := range chapters {
		wg.Add(1)
		go func(chapter *data.Chapter) {
			defer wg.Done()
			semaphore <- struct{}{}
			defer func() { <-semaphore }()

			if err := d.DownloadChapter(manga, chapter); err != nil {
				errorChan <- fmt.Errorf("chapter %s: %w", chapter.Number, err)
				d.sendProgress(DownloadProgress{
					MangaID:       manga.ID,
					ChapterID:     chapter.ID,
					ChapterNumber: chapter.Number,
					Status:        "error",
					Error:         err,
				})
			}
		}(chapter)
	}

	wg.Wait()
	close(errorChan)

	// Check for errors
	var downloadErrors []error
	for err := range errorChan {
		downloadErrors = append(downloadErrors, err)
	}

	if len(downloadErrors) > 0 {
		manga.Status = "partial"
	} else {
		manga.Status = "completed"
	}
	d.repo.SaveManga(manga)

	return nil
}

// DownloadChapter downloads a single chapter
func (d *Downloader) DownloadChapter(manga *data.Manga, chapter *data.Chapter) error {
	<-d.rateLimiter.C // Rate limiting

	d.sendProgress(DownloadProgress{
		MangaID:       manga.ID,
		ChapterID:     chapter.ID,
		ChapterNumber: chapter.Number,
		Status:        "downloading",
	})

	// Get page URLs
	pages, err := d.source.GetPages(manga, chapter)
	if err != nil {
		return fmt.Errorf("failed to get pages: %w", err)
	}

	// Create chapter directory
	chapterDir := filepath.Join(d.downloadDir, manga.ID, chapter.ID)
	if err := os.MkdirAll(chapterDir, 0755); err != nil {
		return fmt.Errorf("failed to create chapter directory: %w", err)
	}

	// Download images
	for i, pageURL := range pages {
		d.sendProgress(DownloadProgress{
			MangaID:       manga.ID,
			ChapterID:     chapter.ID,
			ChapterNumber: chapter.Number,
			CurrentPage:   i + 1,
			TotalPages:    len(pages),
			Status:        "downloading",
		})

		if err := d.downloadImage(pageURL, chapterDir, i); err != nil {
			return fmt.Errorf("failed to download page %d: %w", i, err)
		}

		<-d.rateLimiter.C // Rate limiting between pages
	}

	// Update chapter status
	chapter.Downloaded = true
	chapter.FilePath = chapterDir
	if err := d.repo.UpdateChapterStatus(chapter.ID, true, chapterDir); err != nil {
		return fmt.Errorf("failed to update chapter status: %w", err)
	}

	d.sendProgress(DownloadProgress{
		MangaID:       manga.ID,
		ChapterID:     chapter.ID,
		ChapterNumber: chapter.Number,
		TotalPages:    len(pages),
		Status:        "complete",
	})

	return nil
}

// downloadImage downloads a single image
func (d *Downloader) downloadImage(url, dir string, index int) error {
	resp, err := d.client.Get(url)
	if err != nil {
		return fmt.Errorf("failed to fetch image: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return fmt.Errorf("bad status: %s", resp.Status)
	}

	// Determine file extension from content type or URL
	ext := ".jpg"
	if contentType := resp.Header.Get("Content-Type"); contentType != "" {
		switch contentType {
		case "image/png":
			ext = ".png"
		case "image/gif":
			ext = ".gif"
		case "image/webp":
			ext = ".webp"
		}
	}

	// Create file with zero-padded index
	filename := fmt.Sprintf("%04d%s", index, ext)
	filepath := filepath.Join(dir, filename)

	file, err := os.Create(filepath)
	if err != nil {
		return fmt.Errorf("failed to create file: %w", err)
	}
	defer file.Close()

	_, err = io.Copy(file, resp.Body)
	if err != nil {
		return fmt.Errorf("failed to write file: %w", err)
	}

	return nil
}

// ComposeEPUB creates an EPUB from downloaded chapters
func (d *Downloader) ComposeEPUB(mangaID string) (string, error) {
	manga, err := d.repo.GetManga(mangaID)
	if err != nil {
		return "", fmt.Errorf("failed to get manga: %w", err)
	}
	if manga == nil {
		return "", fmt.Errorf("manga not found")
	}

	chapters, err := d.repo.GetChapters(mangaID)
	if err != nil {
		return "", fmt.Errorf("failed to get chapters: %w", err)
	}

	// Filter only downloaded chapters
	var downloadedChapters []*data.Chapter
	for _, ch := range chapters {
		if ch.Downloaded {
			downloadedChapters = append(downloadedChapters, ch)
		}
	}

	if len(downloadedChapters) == 0 {
		return "", fmt.Errorf("no downloaded chapters found")
	}

	d.sendProgress(DownloadProgress{
		MangaID: manga.ID,
		Status:  "processing",
	})

	// Create EPUB
	epubPath, err := d.epubBuilder.CreateEPub(manga, downloadedChapters)
	if err != nil {
		return "", fmt.Errorf("failed to create EPUB: %w", err)
	}

	manga.Status = "completed"
	d.repo.SaveManga(manga)

	d.sendProgress(DownloadProgress{
		MangaID: manga.ID,
		Status:  "complete",
	})

	return epubPath, nil
}

// sendProgress sends a progress update (non-blocking)
func (d *Downloader) sendProgress(progress DownloadProgress) {
	select {
	case d.progressChan <- progress:
	default:
		// Channel full, skip this update
	}
}

// Close cleans up resources
func (d *Downloader) Close() {
	d.rateLimiter.Stop()
	close(d.progressChan)
}
