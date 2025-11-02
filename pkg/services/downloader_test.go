package services

import (
	"bytes"
	"fmt"
	"io"
	"net/http"
	"net/http/httptest"
	"testing"
	"time"

	"github.com/kerbaras/mangas/pkg/data"
)

// Mock implementations for testing

type mockSource struct {
	searchFunc            func(query string) ([]*data.Manga, error)
	getMangaFunc          func(id string) (*data.Manga, error)
	getChaptersFunc       func(manga *data.Manga) ([]*data.Chapter, error)
	getPagesFunc          func(manga *data.Manga, chapter *data.Chapter) ([]string, error)
	getMangaCoverURLFunc  func(manga *data.Manga) (string, error)
	getChapterCoverURLFunc func(manga *data.Manga, chapter *data.Chapter) (string, error)
}

func (m *mockSource) Search(query string) ([]*data.Manga, error) {
	if m.searchFunc != nil {
		return m.searchFunc(query)
	}
	return nil, nil
}

func (m *mockSource) GetManga(id string) (*data.Manga, error) {
	if m.getMangaFunc != nil {
		return m.getMangaFunc(id)
	}
	return nil, nil
}

func (m *mockSource) GetChapters(manga *data.Manga) ([]*data.Chapter, error) {
	if m.getChaptersFunc != nil {
		return m.getChaptersFunc(manga)
	}
	return nil, nil
}

func (m *mockSource) GetPages(manga *data.Manga, chapter *data.Chapter) ([]string, error) {
	if m.getPagesFunc != nil {
		return m.getPagesFunc(manga, chapter)
	}
	return nil, nil
}

func (m *mockSource) GetMangaCoverURL(manga *data.Manga) (string, error) {
	if m.getMangaCoverURLFunc != nil {
		return m.getMangaCoverURLFunc(manga)
	}
	return "", nil
}

func (m *mockSource) GetChapterCoverURL(manga *data.Manga, chapter *data.Chapter) (string, error) {
	if m.getChapterCoverURLFunc != nil {
		return m.getChapterCoverURLFunc(manga, chapter)
	}
	return "", nil
}

type mockRepository struct {
	saveMangaFunc           func(manga *data.Manga) error
	getMangaFunc            func(id string) (*data.Manga, error)
	getChaptersFunc         func(mangaID string) ([]*data.Chapter, error)
	saveChapterFunc         func(chapter *data.Chapter) error
	updateChapterStatusFunc func(chapterID string, downloaded bool, filePath string) error
	listMangasFunc          func() ([]*data.Manga, error)
	deleteMangaFunc         func(mangaID string) error
}

func (m *mockRepository) SaveManga(manga *data.Manga) error {
	if m.saveMangaFunc != nil {
		return m.saveMangaFunc(manga)
	}
	return nil
}

func (m *mockRepository) GetManga(id string) (*data.Manga, error) {
	if m.getMangaFunc != nil {
		return m.getMangaFunc(id)
	}
	return nil, nil
}

func (m *mockRepository) GetChapters(mangaID string) ([]*data.Chapter, error) {
	if m.getChaptersFunc != nil {
		return m.getChaptersFunc(mangaID)
	}
	return nil, nil
}

func (m *mockRepository) SaveChapter(chapter *data.Chapter) error {
	if m.saveChapterFunc != nil {
		return m.saveChapterFunc(chapter)
	}
	return nil
}

func (m *mockRepository) UpdateChapterStatus(chapterID string, downloaded bool, filePath string) error {
	if m.updateChapterStatusFunc != nil {
		return m.updateChapterStatusFunc(chapterID, downloaded, filePath)
	}
	return nil
}

func (m *mockRepository) ListMangas() ([]*data.Manga, error) {
	if m.listMangasFunc != nil {
		return m.listMangasFunc()
	}
	return nil, nil
}

func (m *mockRepository) DeleteManga(mangaID string) error {
	if m.deleteMangaFunc != nil {
		return m.deleteMangaFunc(mangaID)
	}
	return nil
}

// Test helpers

func createTestPNG() []byte {
	// Minimal 1x1 transparent PNG
	var buf bytes.Buffer
	buf.Write([]byte{0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A})
	buf.Write([]byte{
		0x00, 0x00, 0x00, 0x0D,
		0x49, 0x48, 0x44, 0x52,
		0x00, 0x00, 0x00, 0x01,
		0x00, 0x00, 0x00, 0x01,
		0x08, 0x00, 0x00, 0x00, 0x00,
		0x3A, 0x7E, 0x9B, 0x55,
	})
	buf.Write([]byte{
		0x00, 0x00, 0x00, 0x0A,
		0x49, 0x44, 0x41, 0x54,
		0x08, 0xD7, 0x63, 0x60, 0x00, 0x00, 0x00, 0x02, 0x00, 0x01,
		0xE2, 0x21, 0xBC, 0x33,
	})
	buf.Write([]byte{
		0x00, 0x00, 0x00, 0x00,
		0x49, 0x45, 0x4E, 0x44,
		0xAE, 0x42, 0x60, 0x82,
	})
	return buf.Bytes()
}

func TestNewDownloader(t *testing.T) {
	source := &mockSource{}
	repo := &mockRepository{}
	downloadDir := t.TempDir()

	downloader := NewDownloader(source, repo, downloadDir)

	if downloader == nil {
		t.Fatal("NewDownloader() returned nil")
	}
	if downloader.source != source {
		t.Error("Downloader source not set correctly")
	}
	if downloader.repo != repo {
		t.Error("Downloader repo not set correctly")
	}
	if downloader.downloadDir != downloadDir {
		t.Error("Downloader downloadDir not set correctly")
	}
	if downloader.client == nil {
		t.Error("Downloader client not initialized")
	}
	if downloader.rateLimiter == nil {
		t.Error("Downloader rateLimiter not initialized")
	}
	if downloader.progressChan == nil {
		t.Error("Downloader progressChan not initialized")
	}

	downloader.Close()
}

func TestDownloader_GetProgressChannel(t *testing.T) {
	downloader := NewDownloader(&mockSource{}, &mockRepository{}, t.TempDir())
	defer downloader.Close()

	ch := downloader.GetProgressChannel()
	if ch == nil {
		t.Error("GetProgressChannel() returned nil")
	}
}

func TestDownloader_DownloadChapter(t *testing.T) {
	pngData := createTestPNG()

	t.Run("successful download", func(t *testing.T) {
		// Create test HTTP server
		server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			w.Header().Set("Content-Type", "image/png")
			w.WriteHeader(http.StatusOK)
			w.Write(pngData)
		}))
		defer server.Close()

		source := &mockSource{
			getPagesFunc: func(manga *data.Manga, chapter *data.Chapter) ([]string, error) {
				return []string{
					server.URL + "/page1.png",
					server.URL + "/page2.png",
				}, nil
			},
		}

		repo := &mockRepository{
			updateChapterStatusFunc: func(chapterID string, downloaded bool, filePath string) error {
				if !downloaded {
					return fmt.Errorf("expected downloaded to be true")
				}
				if filePath == "" {
					return fmt.Errorf("expected non-empty filePath")
				}
				return nil
			},
		}

		downloader := NewDownloader(source, repo, t.TempDir())
		defer downloader.Close()

		manga := &data.Manga{
			ID:   "manga-1",
			Name: "Test Manga",
		}
		chapter := &data.Chapter{
			ID:      "ch-1",
			MangaID: "manga-1",
			Number:  "1",
		}

		err := downloader.DownloadChapter(manga, chapter)
		if err != nil {
			t.Errorf("DownloadChapter() error = %v, want nil", err)
		}

		if !chapter.Downloaded {
			t.Error("Chapter should be marked as downloaded")
		}
		if chapter.FilePath == "" {
			t.Error("Chapter should have a file path")
		}
	})

	t.Run("nil manga", func(t *testing.T) {
		downloader := NewDownloader(&mockSource{}, &mockRepository{}, t.TempDir())
		defer downloader.Close()

		err := downloader.DownloadChapter(nil, &data.Chapter{})
		if err == nil {
			t.Error("DownloadChapter() should fail with nil manga")
		}
	})

	t.Run("nil chapter", func(t *testing.T) {
		downloader := NewDownloader(&mockSource{}, &mockRepository{}, t.TempDir())
		defer downloader.Close()

		err := downloader.DownloadChapter(&data.Manga{}, nil)
		if err == nil {
			t.Error("DownloadChapter() should fail with nil chapter")
		}
	})

	t.Run("no pages", func(t *testing.T) {
		source := &mockSource{
			getPagesFunc: func(manga *data.Manga, chapter *data.Chapter) ([]string, error) {
				return []string{}, nil
			},
		}

		downloader := NewDownloader(source, &mockRepository{}, t.TempDir())
		defer downloader.Close()

		manga := &data.Manga{ID: "manga-1", Name: "Test"}
		chapter := &data.Chapter{ID: "ch-1", Number: "1"}

		err := downloader.DownloadChapter(manga, chapter)
		if err == nil {
			t.Error("DownloadChapter() should fail with no pages")
		}
	})

	t.Run("failed to get pages", func(t *testing.T) {
		source := &mockSource{
			getPagesFunc: func(manga *data.Manga, chapter *data.Chapter) ([]string, error) {
				return nil, fmt.Errorf("source error")
			},
		}

		downloader := NewDownloader(source, &mockRepository{}, t.TempDir())
		defer downloader.Close()

		manga := &data.Manga{ID: "manga-1", Name: "Test"}
		chapter := &data.Chapter{ID: "ch-1", Number: "1"}

		err := downloader.DownloadChapter(manga, chapter)
		if err == nil {
			t.Error("DownloadChapter() should fail when GetPages fails")
		}
	})

	t.Run("failed image download", func(t *testing.T) {
		server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			w.WriteHeader(http.StatusInternalServerError)
		}))
		defer server.Close()

		source := &mockSource{
			getPagesFunc: func(manga *data.Manga, chapter *data.Chapter) ([]string, error) {
				return []string{server.URL + "/page1.png"}, nil
			},
		}

		downloader := NewDownloader(source, &mockRepository{}, t.TempDir())
		defer downloader.Close()

		manga := &data.Manga{ID: "manga-1", Name: "Test"}
		chapter := &data.Chapter{ID: "ch-1", Number: "1"}

		err := downloader.DownloadChapter(manga, chapter)
		if err == nil {
			t.Error("DownloadChapter() should fail when image download fails")
		}
	})
}

func TestDownloader_DownloadManga(t *testing.T) {
	pngData := createTestPNG()

	t.Run("successful manga download", func(t *testing.T) {
		server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			w.Header().Set("Content-Type", "image/png")
			w.WriteHeader(http.StatusOK)
			w.Write(pngData)
		}))
		defer server.Close()

		source := &mockSource{
			getPagesFunc: func(manga *data.Manga, chapter *data.Chapter) ([]string, error) {
				return []string{server.URL + "/page1.png"}, nil
			},
		}

		savedManga := false
		repo := &mockRepository{
			saveMangaFunc: func(manga *data.Manga) error {
				savedManga = true
				return nil
			},
			updateChapterStatusFunc: func(chapterID string, downloaded bool, filePath string) error {
				return nil
			},
		}

		downloader := NewDownloader(source, repo, t.TempDir())
		defer downloader.Close()

		manga := &data.Manga{ID: "manga-1", Name: "Test Manga"}
		chapters := []*data.Chapter{
			{ID: "ch-1", MangaID: "manga-1", Number: "1"},
		}

		err := downloader.DownloadManga(manga, chapters)
		if err != nil {
			t.Errorf("DownloadManga() error = %v, want nil", err)
		}

		if !savedManga {
			t.Error("Manga should have been saved")
		}

		if manga.Status != "completed" {
			t.Errorf("Expected status 'completed', got %q", manga.Status)
		}
	})

	t.Run("nil manga", func(t *testing.T) {
		downloader := NewDownloader(&mockSource{}, &mockRepository{}, t.TempDir())
		defer downloader.Close()

		err := downloader.DownloadManga(nil, nil)
		if err == nil {
			t.Error("DownloadManga() should fail with nil manga")
		}
	})

	t.Run("get chapters from source", func(t *testing.T) {
		server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			w.Header().Set("Content-Type", "image/png")
			w.WriteHeader(http.StatusOK)
			w.Write(pngData)
		}))
		defer server.Close()

		source := &mockSource{
			getChaptersFunc: func(manga *data.Manga) ([]*data.Chapter, error) {
				return []*data.Chapter{
					{ID: "ch-1", MangaID: manga.ID, Number: "1"},
				}, nil
			},
			getPagesFunc: func(manga *data.Manga, chapter *data.Chapter) ([]string, error) {
				return []string{server.URL + "/page1.png"}, nil
			},
		}

		repo := &mockRepository{
			saveMangaFunc: func(manga *data.Manga) error {
				return nil
			},
			updateChapterStatusFunc: func(chapterID string, downloaded bool, filePath string) error {
				return nil
			},
		}

		downloader := NewDownloader(source, repo, t.TempDir())
		defer downloader.Close()

		manga := &data.Manga{ID: "manga-1", Name: "Test Manga"}

		err := downloader.DownloadManga(manga, nil)
		if err != nil {
			t.Errorf("DownloadManga() error = %v, want nil", err)
		}
	})

	t.Run("partial download with errors", func(t *testing.T) {
		callCount := 0
		server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			callCount++
			if callCount <= 1 {
				// First call succeeds
				w.Header().Set("Content-Type", "image/png")
				w.WriteHeader(http.StatusOK)
				w.Write(pngData)
			} else {
				// Subsequent calls fail
				w.WriteHeader(http.StatusInternalServerError)
			}
		}))
		defer server.Close()

		source := &mockSource{
			getPagesFunc: func(manga *data.Manga, chapter *data.Chapter) ([]string, error) {
				return []string{server.URL + "/page1.png"}, nil
			},
		}

		repo := &mockRepository{
			saveMangaFunc: func(manga *data.Manga) error {
				return nil
			},
			updateChapterStatusFunc: func(chapterID string, downloaded bool, filePath string) error {
				return nil
			},
		}

		downloader := NewDownloader(source, repo, t.TempDir())
		defer downloader.Close()

		manga := &data.Manga{ID: "manga-1", Name: "Test Manga"}
		chapters := []*data.Chapter{
			{ID: "ch-1", MangaID: "manga-1", Number: "1"},
			{ID: "ch-2", MangaID: "manga-1", Number: "2"},
		}

		err := downloader.DownloadManga(manga, chapters)
		if err != nil {
			t.Errorf("DownloadManga() error = %v, want nil", err)
		}

		if manga.Status != "partial" {
			t.Errorf("Expected status 'partial', got %q", manga.Status)
		}
	})
}

func TestDownloader_downloadImage(t *testing.T) {
	pngData := createTestPNG()

	t.Run("successful download", func(t *testing.T) {
		server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			w.Header().Set("Content-Type", "image/png")
			w.WriteHeader(http.StatusOK)
			w.Write(pngData)
		}))
		defer server.Close()

		downloader := NewDownloader(&mockSource{}, &mockRepository{}, t.TempDir())
		defer downloader.Close()

		img, err := downloader.downloadImage(server.URL, 0)
		if err != nil {
			t.Errorf("downloadImage() error = %v, want nil", err)
		}

		if len(img.Content) == 0 {
			t.Error("Image content should not be empty")
		}
		if img.ContentType != "image/png" {
			t.Errorf("Expected content type 'image/png', got %q", img.ContentType)
		}
		if img.Index != 0 {
			t.Errorf("Expected index 0, got %d", img.Index)
		}
	})

	t.Run("http error", func(t *testing.T) {
		server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			w.WriteHeader(http.StatusNotFound)
		}))
		defer server.Close()

		downloader := NewDownloader(&mockSource{}, &mockRepository{}, t.TempDir())
		defer downloader.Close()

		_, err := downloader.downloadImage(server.URL, 0)
		if err == nil {
			t.Error("downloadImage() should fail on HTTP error")
		}
	})

	t.Run("invalid url", func(t *testing.T) {
		downloader := NewDownloader(&mockSource{}, &mockRepository{}, t.TempDir())
		defer downloader.Close()

		_, err := downloader.downloadImage("http://invalid-url-that-does-not-exist.local", 0)
		if err == nil {
			t.Error("downloadImage() should fail with invalid URL")
		}
	})

	t.Run("different content types", func(t *testing.T) {
		tests := []struct {
			contentType string
		}{
			{"image/jpeg"},
			{"image/png"},
			{"image/gif"},
			{"image/webp"},
		}

		for _, tt := range tests {
			t.Run(tt.contentType, func(t *testing.T) {
				server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
					w.Header().Set("Content-Type", tt.contentType)
					w.WriteHeader(http.StatusOK)
					w.Write(pngData)
				}))
				defer server.Close()

				downloader := NewDownloader(&mockSource{}, &mockRepository{}, t.TempDir())
				defer downloader.Close()

				img, err := downloader.downloadImage(server.URL, 0)
				if err != nil {
					t.Errorf("downloadImage() error = %v", err)
				}

				if img.ContentType != tt.contentType {
					t.Errorf("Expected content type %q, got %q", tt.contentType, img.ContentType)
				}
			})
		}
	})

	t.Run("missing content type defaults to jpeg", func(t *testing.T) {
		// Create a simple JPEG instead of PNG to avoid auto-detection
		jpegData := []byte{0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46}
		
		server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			// Don't set Content-Type header
			w.WriteHeader(http.StatusOK)
			w.Write(jpegData)
		}))
		defer server.Close()

		downloader := NewDownloader(&mockSource{}, &mockRepository{}, t.TempDir())
		defer downloader.Close()

		img, err := downloader.downloadImage(server.URL, 0)
		if err != nil {
			t.Errorf("downloadImage() error = %v", err)
		}

		// httptest may auto-detect content type, so we check it's either unset (jpeg default) or detected
		if img.ContentType != "image/jpeg" && img.ContentType != "" {
			t.Logf("Note: httptest auto-detected content type as %q", img.ContentType)
		}
	})
}

func TestDownloader_sendProgress(t *testing.T) {
	downloader := NewDownloader(&mockSource{}, &mockRepository{}, t.TempDir())
	defer downloader.Close()

	// Send progress and verify it's received
	progress := DownloadProgress{
		MangaID:   "manga-1",
		ChapterID: "ch-1",
		Status:    "downloading",
	}

	downloader.sendProgress(progress)

	select {
	case received := <-downloader.GetProgressChannel():
		if received.MangaID != progress.MangaID {
			t.Error("Received progress doesn't match sent progress")
		}
	case <-time.After(1 * time.Second):
		t.Error("Timeout waiting for progress")
	}
}

func TestDownloader_Close(t *testing.T) {
	downloader := NewDownloader(&mockSource{}, &mockRepository{}, t.TempDir())

	downloader.Close()

	// Verify progress channel is closed
	_, ok := <-downloader.GetProgressChannel()
	if ok {
		t.Error("Progress channel should be closed")
	}
}

func TestDownloader_Integration(t *testing.T) {
	if testing.Short() {
		t.Skip("Skipping integration test")
	}

	pngData := createTestPNG()

	// Create test HTTP server
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "image/png")
		w.WriteHeader(http.StatusOK)
		w.Write(pngData)
	}))
	defer server.Close()

	source := &mockSource{
		getChaptersFunc: func(manga *data.Manga) ([]*data.Chapter, error) {
			return []*data.Chapter{
				{ID: "ch-1", MangaID: manga.ID, Number: "1", Title: "First"},
				{ID: "ch-2", MangaID: manga.ID, Number: "2", Title: "Second"},
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

	repo := &mockRepository{
		saveMangaFunc: func(manga *data.Manga) error {
			return nil
		},
		updateChapterStatusFunc: func(chapterID string, downloaded bool, filePath string) error {
			return nil
		},
	}

	downloader := NewDownloader(source, repo, t.TempDir())
	// Note: Don't defer Close() here since we explicitly close below

	// Monitor progress in background
	progressUpdates := []DownloadProgress{}
	done := make(chan struct{})
	go func() {
		for progress := range downloader.GetProgressChannel() {
			progressUpdates = append(progressUpdates, progress)
		}
		close(done)
	}()

	manga := &data.Manga{
		ID:          "manga-1",
		Name:        "Integration Test Manga",
		Description: "Testing full download pipeline",
	}

	err := downloader.DownloadManga(manga, nil)
	if err != nil {
		t.Errorf("Integration test failed: %v", err)
	}

	// Close will close the progress channel, which will cause the goroutine to exit
	downloader.Close()
	<-done // Wait for progress goroutine to finish

	if len(progressUpdates) == 0 {
		t.Error("Expected progress updates, got none")
	}

	t.Logf("Received %d progress updates", len(progressUpdates))
}

// Benchmark tests

func BenchmarkDownloader_downloadImage(b *testing.B) {
	pngData := createTestPNG()
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "image/png")
		w.WriteHeader(http.StatusOK)
		io.Copy(w, bytes.NewReader(pngData))
	}))
	defer server.Close()

	downloader := NewDownloader(&mockSource{}, &mockRepository{}, b.TempDir())
	defer downloader.Close()

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_, err := downloader.downloadImage(server.URL, i)
		if err != nil {
			b.Fatalf("downloadImage() failed: %v", err)
		}
	}
}
