package services

import (
	"fmt"
	"io"
	"net/http"
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
	ListMangas() ([]*data.Manga, error)
	DeleteManga(mangaID string) error
}

// Downloader orchestrates manga downloads as a streaming pipeline
type Downloader struct {
	source       sources.Source
	repo         Repository
	downloadDir  string
	client       *http.Client
	rateLimiter  *time.Ticker
	progressChan chan DownloadProgress
}

// NewDownloader creates a new Downloader instance
func NewDownloader(source sources.Source, repo Repository, downloadDir string) *Downloader {
	return &Downloader{
		source:       source,
		repo:         repo,
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
	if manga == nil {
		return fmt.Errorf("manga cannot be nil")
	}

	// Save manga to database
	manga.Status = "downloading"
	if err := d.repo.SaveManga(manga); err != nil {
		return fmt.Errorf("failed to save manga: %w", err)
	}

	// Get chapters if not provided
	if len(chapters) == 0 {
		var err error
		chapters, err = d.source.GetChapters(manga)
		if err != nil {
			return fmt.Errorf("failed to get chapters: %w", err)
		}
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

// DownloadChapter downloads a single chapter and streams it to an EPUB
func (d *Downloader) DownloadChapter(manga *data.Manga, chapter *data.Chapter) error {
	if manga == nil {
		return fmt.Errorf("manga cannot be nil")
	}
	if chapter == nil {
		return fmt.Errorf("chapter cannot be nil")
	}

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

	if len(pages) == 0 {
		return fmt.Errorf("no pages found for chapter")
	}

	// Initialize EPUB builder
	builder := integrations.NewEPubBuilder(d.downloadDir)
	if err := builder.Init(manga, chapter); err != nil {
		return fmt.Errorf("failed to initialize EPUB builder: %w", err)
	}

	// Download and set manga cover
	mangaCoverURL, err := d.source.GetMangaCoverURL(manga)
	if err == nil && mangaCoverURL != "" {
		coverData, err := d.downloadCoverImage(mangaCoverURL)
		if err == nil {
			builder.SetMangaCover(coverData)
		}
		// Non-fatal error, continue even if cover download fails
		<-d.rateLimiter.C // Rate limiting
	}

	// Download and set chapter cover (if different from manga cover)
	chapterCoverURL, err := d.source.GetChapterCoverURL(manga, chapter)
	if err == nil && chapterCoverURL != "" && chapterCoverURL != mangaCoverURL {
		coverData, err := d.downloadCoverImage(chapterCoverURL)
		if err == nil {
			builder.SetChapterCover(coverData)
		}
		// Non-fatal error, continue even if cover download fails
		<-d.rateLimiter.C // Rate limiting
	}

	d.sendProgress(DownloadProgress{
		MangaID:       manga.ID,
		ChapterID:     chapter.ID,
		ChapterNumber: chapter.Number,
		TotalPages:    len(pages),
		Status:        "downloading",
	})

	// Stream images to EPUB builder
	for i, pageURL := range pages {
		d.sendProgress(DownloadProgress{
			MangaID:       manga.ID,
			ChapterID:     chapter.ID,
			ChapterNumber: chapter.Number,
			CurrentPage:   i + 1,
			TotalPages:    len(pages),
			Status:        "downloading",
		})

		imageData, err := d.downloadImage(pageURL, i)
		if err != nil {
			return fmt.Errorf("failed to download page %d: %w", i, err)
		}

		// Stream image to builder
		if err := builder.Next(imageData); err != nil {
			return fmt.Errorf("failed to add page %d to EPUB: %w", i, err)
		}

		<-d.rateLimiter.C // Rate limiting between pages
	}

	// Finalize EPUB
	d.sendProgress(DownloadProgress{
		MangaID:       manga.ID,
		ChapterID:     chapter.ID,
		ChapterNumber: chapter.Number,
		TotalPages:    len(pages),
		Status:        "processing",
	})

	epubPath, err := builder.Done()
	if err != nil {
		return fmt.Errorf("failed to finalize EPUB: %w", err)
	}

	// Update chapter status
	chapter.Downloaded = true
	chapter.FilePath = epubPath
	if err := d.repo.UpdateChapterStatus(chapter.ID, true, epubPath); err != nil {
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

// downloadImage downloads a single image and returns its data
func (d *Downloader) downloadImage(url string, index int) (integrations.ImageData, error) {
	resp, err := d.client.Get(url)
	if err != nil {
		return integrations.ImageData{}, fmt.Errorf("failed to fetch image: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return integrations.ImageData{}, fmt.Errorf("bad status: %s", resp.Status)
	}

	// Read image content into memory
	content, err := io.ReadAll(resp.Body)
	if err != nil {
		return integrations.ImageData{}, fmt.Errorf("failed to read image content: %w", err)
	}

	// Determine content type
	contentType := resp.Header.Get("Content-Type")
	if contentType == "" {
		contentType = "image/jpeg" // Default to JPEG
	}

	return integrations.ImageData{
		Content:     content,
		ContentType: contentType,
		Index:       index,
	}, nil
}

// downloadCoverImage downloads a cover image and returns its data
func (d *Downloader) downloadCoverImage(url string) (integrations.CoverData, error) {
	resp, err := d.client.Get(url)
	if err != nil {
		return integrations.CoverData{}, fmt.Errorf("failed to fetch cover image: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return integrations.CoverData{}, fmt.Errorf("bad status for cover image: %s", resp.Status)
	}

	// Read image content into memory
	content, err := io.ReadAll(resp.Body)
	if err != nil {
		return integrations.CoverData{}, fmt.Errorf("failed to read cover image content: %w", err)
	}

	// Determine content type
	contentType := resp.Header.Get("Content-Type")
	if contentType == "" {
		contentType = "image/jpeg" // Default to JPEG
	}

	return integrations.CoverData{
		Content:     content,
		ContentType: contentType,
	}, nil
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
	
	// Close progress channel safely
	select {
	case <-d.progressChan:
		// Already closed
	default:
		close(d.progressChan)
	}
}
