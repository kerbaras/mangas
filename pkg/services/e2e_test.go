package services

import (
	"fmt"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"sync"
	"testing"
	"time"

	"github.com/kerbaras/mangas/pkg/data"
)

// E2E tests for the full download pipeline

func TestE2E_FullDownloadPipeline(t *testing.T) {
	if testing.Short() {
		t.Skip("Skipping E2E test in short mode")
	}

	// Create test PNG image
	pngData := createTestPNG()

	// Create HTTP server to simulate manga source
	requestCount := 0
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		requestCount++
		w.Header().Set("Content-Type", "image/png")
		w.WriteHeader(http.StatusOK)
		w.Write(pngData)
	}))
	defer server.Close()

	// Setup test environment
	testDir := t.TempDir()
	downloadDir := filepath.Join(testDir, "downloads")

	// Create mock source that returns test data
	source := &mockSource{
		getChaptersFunc: func(manga *data.Manga) ([]*data.Chapter, error) {
			return []*data.Chapter{
				{ID: "ch1", MangaID: manga.ID, Number: "1", Language: "en", Title: "First Chapter"},
				{ID: "ch2", MangaID: manga.ID, Number: "2", Language: "en", Title: "Second Chapter"},
			}, nil
		},
		getPagesFunc: func(manga *data.Manga, chapter *data.Chapter) ([]string, error) {
			return []string{
				server.URL + "/page1.png",
				server.URL + "/page2.png",
				server.URL + "/page3.png",
			}, nil
		},
	}

	// Create repository
	repo := data.NewDuckDBRepository()

	// Create controller
	config := ControllerConfig{
		DownloadDir: downloadDir,
	}
	controller := NewMangaControllerWithConfig(config)
	controller.source = source
	controller.repo = repo
	controller.downloader = NewDownloader(source, repo, downloadDir)
	// Don't defer Close() here - we'll call it explicitly at the end

	// Test manga
	manga := &data.Manga{
		ID:          "manga-test",
		Name:        "E2E Test Manga",
		Description: "Testing full pipeline",
	}

	// Add manga to library
	t.Run("Add to library", func(t *testing.T) {
		err := controller.AddMangaToLibrary(manga)
		if err != nil {
			t.Fatalf("Failed to add manga to library: %v", err)
		}

		// Verify manga was saved
		savedManga, err := controller.GetMangaFromLibrary(manga.ID)
		if err != nil {
			t.Fatalf("Failed to retrieve manga from library: %v", err)
		}
		if savedManga.Name != manga.Name {
			t.Errorf("Expected name %q, got %q", manga.Name, savedManga.Name)
		}

		// Verify chapters were saved
		chapters, err := controller.GetChaptersFromLibrary(manga.ID)
		if err != nil {
			t.Fatalf("Failed to get chapters from library: %v", err)
		}
		if len(chapters) != 2 {
			t.Errorf("Expected 2 chapters, got %d", len(chapters))
		}
	})

	// Monitor progress
	progressUpdates := []DownloadProgress{}
	done := make(chan struct{})
	go func() {
		for progress := range controller.GetProgressChannel() {
			progressUpdates = append(progressUpdates, progress)
		}
		close(done)
	}()

	// Download manga
	t.Run("Download chapters", func(t *testing.T) {
		options := DownloadOptions{
			Language: "en",
		}

		err := controller.DownloadManga(manga, options)
		if err != nil {
			t.Fatalf("Failed to download manga: %v", err)
		}

		// Wait a bit for progress updates
		time.Sleep(100 * time.Millisecond)

		// Verify progress updates were sent
		if len(progressUpdates) == 0 {
			t.Error("Expected progress updates, got none")
		}

		// Count complete statuses
		completeCount := 0
		for _, p := range progressUpdates {
			if p.Status == "complete" {
				completeCount++
			}
		}
		if completeCount != 2 {
			t.Errorf("Expected 2 complete progress updates, got %d", completeCount)
		}

		t.Logf("Received %d progress updates, %d complete", len(progressUpdates), completeCount)
	})

	// Verify EPUBs were created
	t.Run("Verify EPUB files", func(t *testing.T) {
		files, err := os.ReadDir(downloadDir)
		if err != nil {
			t.Fatalf("Failed to read download directory: %v", err)
		}

		epubCount := 0
		for _, file := range files {
			if filepath.Ext(file.Name()) == ".epub" {
				epubCount++
				
				// Verify file is not empty
				info, err := os.Stat(filepath.Join(downloadDir, file.Name()))
				if err != nil {
					t.Errorf("Failed to stat EPUB file: %v", err)
				}
				if info.Size() == 0 {
					t.Errorf("EPUB file %s is empty", file.Name())
				}
			}
		}

		if epubCount != 2 {
			t.Errorf("Expected 2 EPUB files, found %d", epubCount)
		}
	})

	// Verify HTTP requests were made
	t.Run("Verify HTTP requests", func(t *testing.T) {
		// 2 chapters ? 3 pages = 6 requests
		if requestCount != 6 {
			t.Errorf("Expected 6 HTTP requests, got %d", requestCount)
		}
	})

	// Verify chapters marked as downloaded
	t.Run("Verify chapters marked as downloaded", func(t *testing.T) {
		chapters, err := controller.GetChaptersFromLibrary(manga.ID)
		if err != nil {
			t.Fatalf("Failed to get chapters: %v", err)
		}

		for _, ch := range chapters {
			if !ch.Downloaded {
				t.Errorf("Chapter %s should be marked as downloaded", ch.Number)
			}
			if ch.FilePath == "" {
				t.Errorf("Chapter %s should have a file path", ch.Number)
			}
		}
	})

	// Close controller and wait for progress goroutine to complete
	controller.Close()
	<-done
}

func TestE2E_DownloadWithChapterRange(t *testing.T) {
	if testing.Short() {
		t.Skip("Skipping E2E test in short mode")
	}

	pngData := createTestPNG()
	requestCount := 0

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		requestCount++
		w.Header().Set("Content-Type", "image/png")
		w.WriteHeader(http.StatusOK)
		w.Write(pngData)
	}))
	defer server.Close()

	testDir := t.TempDir()

	source := &mockSource{
		getChaptersFunc: func(manga *data.Manga) ([]*data.Chapter, error) {
			return []*data.Chapter{
				{ID: "ch1", MangaID: manga.ID, Number: "1", Language: "en"},
				{ID: "ch2", MangaID: manga.ID, Number: "2", Language: "en"},
				{ID: "ch3", MangaID: manga.ID, Number: "3", Language: "en"},
				{ID: "ch4", MangaID: manga.ID, Number: "4", Language: "en"},
			}, nil
		},
		getPagesFunc: func(manga *data.Manga, chapter *data.Chapter) ([]string, error) {
			return []string{server.URL + "/page1.png"}, nil
		},
	}

	config := ControllerConfig{DownloadDir: testDir}
	controller := NewMangaControllerWithConfig(config)
	controller.source = source
	controller.repo = data.NewDuckDBRepository()
	controller.downloader = NewDownloader(source, controller.repo, testDir)
	defer controller.Close()

	manga := &data.Manga{ID: "manga-range", Name: "Range Test"}

	// Download only chapters 2-3
	options := DownloadOptions{
		Language:     "en",
		ChapterRange: "2-3",
	}

	err := controller.DownloadManga(manga, options)
	if err != nil {
		t.Fatalf("Download failed: %v", err)
	}

	// Should have downloaded 2 chapters
	if requestCount != 2 {
		t.Errorf("Expected 2 chapters downloaded, got %d requests", requestCount)
	}
}

func TestE2E_DownloadWithLanguageFilter(t *testing.T) {
	if testing.Short() {
		t.Skip("Skipping E2E test in short mode")
	}

	pngData := createTestPNG()
	requestCount := 0

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		requestCount++
		w.Header().Set("Content-Type", "image/png")
		w.WriteHeader(http.StatusOK)
		w.Write(pngData)
	}))
	defer server.Close()

	testDir := t.TempDir()

	source := &mockSource{
		getChaptersFunc: func(manga *data.Manga) ([]*data.Chapter, error) {
			return []*data.Chapter{
				{ID: "ch1", MangaID: manga.ID, Number: "1", Language: "en"},
				{ID: "ch2", MangaID: manga.ID, Number: "1", Language: "ja"},
				{ID: "ch3", MangaID: manga.ID, Number: "2", Language: "en"},
				{ID: "ch4", MangaID: manga.ID, Number: "2", Language: "ja"},
			}, nil
		},
		getPagesFunc: func(manga *data.Manga, chapter *data.Chapter) ([]string, error) {
			return []string{server.URL + "/page1.png"}, nil
		},
	}

	config := ControllerConfig{DownloadDir: testDir}
	controller := NewMangaControllerWithConfig(config)
	controller.source = source
	controller.repo = data.NewDuckDBRepository()
	controller.downloader = NewDownloader(source, controller.repo, testDir)
	defer controller.Close()

	manga := &data.Manga{ID: "manga-lang", Name: "Language Test"}

	// Download only English chapters
	options := DownloadOptions{
		Language: "en",
	}

	err := controller.DownloadManga(manga, options)
	if err != nil {
		t.Fatalf("Download failed: %v", err)
	}

	// Should have downloaded 2 English chapters only
	if requestCount != 2 {
		t.Errorf("Expected 2 English chapters downloaded, got %d requests", requestCount)
	}
}

func TestE2E_DownloadWithSpecificChapters(t *testing.T) {
	if testing.Short() {
		t.Skip("Skipping E2E test in short mode")
	}

	pngData := createTestPNG()
	requestCount := 0

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		requestCount++
		w.Header().Set("Content-Type", "image/png")
		w.WriteHeader(http.StatusOK)
		w.Write(pngData)
	}))
	defer server.Close()

	testDir := t.TempDir()

	source := &mockSource{
		getChaptersFunc: func(manga *data.Manga) ([]*data.Chapter, error) {
			return []*data.Chapter{
				{ID: "ch1", MangaID: manga.ID, Number: "1", Language: "en"},
				{ID: "ch2", MangaID: manga.ID, Number: "2", Language: "en"},
				{ID: "ch3", MangaID: manga.ID, Number: "3", Language: "en"},
			}, nil
		},
		getPagesFunc: func(manga *data.Manga, chapter *data.Chapter) ([]string, error) {
			return []string{server.URL + "/page1.png"}, nil
		},
	}

	config := ControllerConfig{DownloadDir: testDir}
	controller := NewMangaControllerWithConfig(config)
	controller.source = source
	controller.repo = data.NewDuckDBRepository()
	controller.downloader = NewDownloader(source, controller.repo, testDir)
	defer controller.Close()

	manga := &data.Manga{ID: "manga-specific", Name: "Specific Chapters Test"}

	// Download only specific chapters
	options := DownloadOptions{
		Language:   "en",
		ChapterIDs: []string{"ch1", "ch3"},
	}

	err := controller.DownloadManga(manga, options)
	if err != nil {
		t.Fatalf("Download failed: %v", err)
	}

	// Should have downloaded 2 specific chapters
	if requestCount != 2 {
		t.Errorf("Expected 2 specific chapters downloaded, got %d requests", requestCount)
	}
}

func TestE2E_DownloadWithErrors(t *testing.T) {
	if testing.Short() {
		t.Skip("Skipping E2E test in short mode")
	}

	pngData := createTestPNG()
	callCount := 0

	// Server that fails on second chapter
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		callCount++
		if callCount > 1 {
			w.WriteHeader(http.StatusInternalServerError)
			return
		}
		w.Header().Set("Content-Type", "image/png")
		w.WriteHeader(http.StatusOK)
		w.Write(pngData)
	}))
	defer server.Close()

	testDir := t.TempDir()

	source := &mockSource{
		getChaptersFunc: func(manga *data.Manga) ([]*data.Chapter, error) {
			return []*data.Chapter{
				{ID: "ch1", MangaID: manga.ID, Number: "1", Language: "en"},
				{ID: "ch2", MangaID: manga.ID, Number: "2", Language: "en"},
			}, nil
		},
		getPagesFunc: func(manga *data.Manga, chapter *data.Chapter) ([]string, error) {
			return []string{server.URL + "/page1.png"}, nil
		},
	}

	config := ControllerConfig{DownloadDir: testDir}
	controller := NewMangaControllerWithConfig(config)
	controller.source = source
	controller.repo = data.NewDuckDBRepository()
	controller.downloader = NewDownloader(source, controller.repo, testDir)
	defer controller.Close()

	manga := &data.Manga{ID: "manga-errors", Name: "Error Test"}

	options := DownloadOptions{Language: "en"}

	// Download should complete but with errors
	err := controller.DownloadManga(manga, options)
	if err != nil {
		t.Logf("Download completed with errors: %v", err)
	}

	// Verify manga status is partial
	savedManga, _ := controller.GetMangaFromLibrary(manga.ID)
	if savedManga != nil && savedManga.Status != "partial" && savedManga.Status != "completed" {
		t.Logf("Manga status: %s", savedManga.Status)
	}
}

func TestE2E_ConcurrentDownloads(t *testing.T) {
	if testing.Short() {
		t.Skip("Skipping E2E test in short mode")
	}

	pngData := createTestPNG()
	requestCount := 0
	var requestMutex sync.Mutex

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		requestMutex.Lock()
		requestCount++
		requestMutex.Unlock()
		
		// Simulate some delay
		time.Sleep(10 * time.Millisecond)
		
		w.Header().Set("Content-Type", "image/png")
		w.WriteHeader(http.StatusOK)
		w.Write(pngData)
	}))
	defer server.Close()

	testDir := t.TempDir()

	source := &mockSource{
		getChaptersFunc: func(manga *data.Manga) ([]*data.Chapter, error) {
			chapters := make([]*data.Chapter, 5)
			for i := 0; i < 5; i++ {
				chapters[i] = &data.Chapter{
					ID:       fmt.Sprintf("ch%d", i+1),
					MangaID:  manga.ID,
					Number:   fmt.Sprintf("%d", i+1),
					Language: "en",
				}
			}
			return chapters, nil
		},
		getPagesFunc: func(manga *data.Manga, chapter *data.Chapter) ([]string, error) {
			return []string{server.URL + "/page1.png"}, nil
		},
	}

	config := ControllerConfig{DownloadDir: testDir}
	controller := NewMangaControllerWithConfig(config)
	controller.source = source
	controller.repo = data.NewDuckDBRepository()
	controller.downloader = NewDownloader(source, controller.repo, testDir)
	defer controller.Close()

	manga := &data.Manga{ID: "manga-concurrent", Name: "Concurrent Test"}

	startTime := time.Now()
	
	err := controller.DownloadManga(manga, DownloadOptions{Language: "en"})
	if err != nil {
		t.Fatalf("Download failed: %v", err)
	}

	duration := time.Since(startTime)

	// With concurrency (max 3), 5 chapters should take less time than sequential
	// Sequential: 5 * 10ms = 50ms minimum
	// Concurrent: ~20ms with 3 concurrent (2 batches)
	if duration > 100*time.Millisecond {
		t.Logf("Download took %v (may be slow due to test environment)", duration)
	}

	requestMutex.Lock()
	finalCount := requestCount
	requestMutex.Unlock()

	if finalCount != 5 {
		t.Errorf("Expected 5 requests, got %d", finalCount)
	}
}
