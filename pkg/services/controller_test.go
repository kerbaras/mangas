package services

import (
	"fmt"
	"os"
	"path/filepath"
	"testing"

	"github.com/kerbaras/mangas/pkg/data"
)

func TestNewMangaController(t *testing.T) {
	controller := NewMangaController()
	
	if controller == nil {
		t.Fatal("NewMangaController() returned nil")
	}
	if controller.source == nil {
		t.Error("Controller source not initialized")
	}
	if controller.repo == nil {
		t.Error("Controller repo not initialized")
	}
	if controller.downloader == nil {
		t.Error("Controller downloader not initialized")
	}
	if controller.downloadDir == "" {
		t.Error("Controller downloadDir not set")
	}

	defer controller.Close()
}

func TestNewMangaControllerWithConfig(t *testing.T) {
	tempDir := t.TempDir()
	
	config := ControllerConfig{
		SourceType:  "mangadex",
		DownloadDir: tempDir,
	}
	
	controller := NewMangaControllerWithConfig(config)
	defer controller.Close()
	
	if controller == nil {
		t.Fatal("NewMangaControllerWithConfig() returned nil")
	}
	if controller.downloadDir != tempDir {
		t.Errorf("Expected downloadDir %s, got %s", tempDir, controller.downloadDir)
	}
	
	// Verify directory was created
	if _, err := os.Stat(tempDir); os.IsNotExist(err) {
		t.Error("Download directory should have been created")
	}
}

func TestControllerSearchManga(t *testing.T) {
	controller := &MangaController{
		source: &mockSource{
			searchFunc: func(query string) ([]*data.Manga, error) {
				if query == "" {
					return nil, fmt.Errorf("empty query")
				}
				return []*data.Manga{
					{ID: "1", Name: "Test Manga"},
				}, nil
			},
		},
	}
	
	t.Run("successful search", func(t *testing.T) {
		results, err := controller.SearchManga("test")
		if err != nil {
			t.Errorf("SearchManga() error = %v, want nil", err)
		}
		if len(results) != 1 {
			t.Errorf("Expected 1 result, got %d", len(results))
		}
	})
	
	t.Run("empty query", func(t *testing.T) {
		_, err := controller.SearchManga("")
		if err == nil {
			t.Error("SearchManga() should fail with empty query")
		}
	})
}

func TestControllerGetManga(t *testing.T) {
	controller := &MangaController{
		source: &mockSource{
			getMangaFunc: func(id string) (*data.Manga, error) {
				if id == "" {
					return nil, fmt.Errorf("empty id")
				}
				return &data.Manga{ID: id, Name: "Test Manga"}, nil
			},
		},
	}
	
	t.Run("successful get", func(t *testing.T) {
		manga, err := controller.GetManga("test-id")
		if err != nil {
			t.Errorf("GetManga() error = %v, want nil", err)
		}
		if manga.ID != "test-id" {
			t.Errorf("Expected ID 'test-id', got %s", manga.ID)
		}
	})
	
	t.Run("empty id", func(t *testing.T) {
		_, err := controller.GetManga("")
		if err == nil {
			t.Error("GetManga() should fail with empty ID")
		}
	})
}

func TestControllerGetMangaFromLibrary(t *testing.T) {
	controller := &MangaController{
		repo: &mockRepository{
			getMangaFunc: func(id string) (*data.Manga, error) {
				if id == "" {
					return nil, fmt.Errorf("empty id")
				}
				return &data.Manga{ID: id, Name: "Library Manga"}, nil
			},
		},
	}
	
	t.Run("successful get", func(t *testing.T) {
		manga, err := controller.GetMangaFromLibrary("lib-id")
		if err != nil {
			t.Errorf("GetMangaFromLibrary() error = %v, want nil", err)
		}
		if manga.Name != "Library Manga" {
			t.Errorf("Expected name 'Library Manga', got %s", manga.Name)
		}
	})
	
	t.Run("empty id", func(t *testing.T) {
		_, err := controller.GetMangaFromLibrary("")
		if err == nil {
			t.Error("GetMangaFromLibrary() should fail with empty ID")
		}
	})
}

func TestControllerFindMangaByName(t *testing.T) {
	controller := &MangaController{
		repo: &mockRepository{
			getMangaFunc: func(id string) (*data.Manga, error) {
				return &data.Manga{ID: id, Name: "Test Manga"}, nil
			},
			saveMangaFunc: func(manga *data.Manga) error {
				return nil
			},
		},
	}
	
	// Setup test data
	controller.repo = &mockRepository{
		getMangaFunc: func(id string) (*data.Manga, error) {
			return &data.Manga{ID: id, Name: "Test Manga"}, nil
		},
		saveMangaFunc: func(manga *data.Manga) error {
			return nil
		},
		getChaptersFunc: func(mangaID string) ([]*data.Chapter, error) {
			return []*data.Chapter{}, nil
		},
	}
	
	// Create real repo for this test
	repo := data.NewDuckDBRepository()
	controller.repo = repo
	
	// Add test manga
	testManga := &data.Manga{
		ID:   "test-id",
		Name: "Test Manga Name",
	}
	repo.SaveManga(testManga)
	
	t.Run("found by exact name", func(t *testing.T) {
		manga, err := controller.FindMangaByName("Test Manga Name")
		if err != nil {
			t.Errorf("FindMangaByName() error = %v, want nil", err)
		}
		if manga.ID != "test-id" {
			t.Errorf("Expected ID 'test-id', got %s", manga.ID)
		}
	})
	
	t.Run("found by case-insensitive name", func(t *testing.T) {
		manga, err := controller.FindMangaByName("test manga name")
		if err != nil {
			t.Errorf("FindMangaByName() error = %v, want nil", err)
		}
		if manga.ID != "test-id" {
			t.Errorf("Expected ID 'test-id', got %s", manga.ID)
		}
	})
	
	t.Run("not found", func(t *testing.T) {
		_, err := controller.FindMangaByName("Nonexistent Manga")
		if err == nil {
			t.Error("FindMangaByName() should fail when manga not found")
		}
	})
	
	t.Run("empty name", func(t *testing.T) {
		_, err := controller.FindMangaByName("")
		if err == nil {
			t.Error("FindMangaByName() should fail with empty name")
		}
	})
}

func TestControllerGetChapters(t *testing.T) {
	controller := &MangaController{
		source: &mockSource{
			getChaptersFunc: func(manga *data.Manga) ([]*data.Chapter, error) {
				if manga == nil {
					return nil, fmt.Errorf("nil manga")
				}
				return []*data.Chapter{
					{ID: "ch1", MangaID: manga.ID, Number: "1"},
					{ID: "ch2", MangaID: manga.ID, Number: "2"},
				}, nil
			},
		},
	}
	
	t.Run("successful get", func(t *testing.T) {
		manga := &data.Manga{ID: "manga-1"}
		chapters, err := controller.GetChapters(manga)
		if err != nil {
			t.Errorf("GetChapters() error = %v, want nil", err)
		}
		if len(chapters) != 2 {
			t.Errorf("Expected 2 chapters, got %d", len(chapters))
		}
	})
	
	t.Run("nil manga", func(t *testing.T) {
		_, err := controller.GetChapters(nil)
		if err == nil {
			t.Error("GetChapters() should fail with nil manga")
		}
	})
}

func TestControllerAddMangaToLibrary(t *testing.T) {
	savedManga := false
	savedChapters := 0
	
	controller := &MangaController{
		source: &mockSource{
			getChaptersFunc: func(manga *data.Manga) ([]*data.Chapter, error) {
				return []*data.Chapter{
					{ID: "ch1", Number: "1"},
					{ID: "ch2", Number: "2"},
				}, nil
			},
		},
		repo: &mockRepository{
			saveMangaFunc: func(manga *data.Manga) error {
				savedManga = true
				return nil
			},
			saveChapterFunc: func(chapter *data.Chapter) error {
				savedChapters++
				return nil
			},
		},
	}
	
	t.Run("successful add", func(t *testing.T) {
		manga := &data.Manga{ID: "manga-1", Name: "Test"}
		err := controller.AddMangaToLibrary(manga)
		if err != nil {
			t.Errorf("AddMangaToLibrary() error = %v, want nil", err)
		}
		if !savedManga {
			t.Error("Manga should have been saved")
		}
		if savedChapters != 2 {
			t.Errorf("Expected 2 chapters saved, got %d", savedChapters)
		}
	})
	
	t.Run("nil manga", func(t *testing.T) {
		err := controller.AddMangaToLibrary(nil)
		if err == nil {
			t.Error("AddMangaToLibrary() should fail with nil manga")
		}
	})
}

func TestControllerFilterChapters(t *testing.T) {
	controller := &MangaController{}
	
	chapters := []*data.Chapter{
		{ID: "1", Number: "1", Language: "en"},
		{ID: "2", Number: "2", Language: "en"},
		{ID: "3", Number: "3", Language: "en"},
		{ID: "4", Number: "1", Language: "ja"},
		{ID: "5", Number: "5", Language: "en"},
	}
	
	t.Run("filter by language", func(t *testing.T) {
		options := DownloadOptions{Language: "en"}
		filtered := controller.filterChapters(chapters, options)
		if len(filtered) != 4 {
			t.Errorf("Expected 4 English chapters, got %d", len(filtered))
		}
	})
	
	t.Run("filter by chapter IDs", func(t *testing.T) {
		options := DownloadOptions{
			Language:   "en",
			ChapterIDs: []string{"1", "3"},
		}
		filtered := controller.filterChapters(chapters, options)
		if len(filtered) != 2 {
			t.Errorf("Expected 2 chapters, got %d", len(filtered))
		}
	})
	
	t.Run("filter by range", func(t *testing.T) {
		options := DownloadOptions{
			Language:     "en",
			ChapterRange: "1-3",
		}
		filtered := controller.filterChapters(chapters, options)
		if len(filtered) != 3 {
			t.Errorf("Expected 3 chapters in range, got %d", len(filtered))
		}
	})
	
	t.Run("no filters", func(t *testing.T) {
		options := DownloadOptions{}
		filtered := controller.filterChapters(chapters, options)
		if len(filtered) != len(chapters) {
			t.Errorf("Expected all %d chapters, got %d", len(chapters), len(filtered))
		}
	})
}

func TestControllerFilterByRange(t *testing.T) {
	controller := &MangaController{}
	
	chapters := []*data.Chapter{
		{ID: "1", Number: "1"},
		{ID: "2", Number: "2.5"},
		{ID: "3", Number: "3"},
		{ID: "4", Number: "5"},
		{ID: "5", Number: "10"},
	}
	
	tests := []struct {
		name     string
		rangeStr string
		expected int
	}{
		{"range 1-3", "1-3", 3},
		{"range 2-5", "2-5", 3},
		{"range 5-10", "5-10", 2},
		{"invalid range", "invalid", 5}, // Should return all
		{"single number", "5", 5},       // Should return all
	}
	
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			filtered := controller.filterByRange(chapters, tt.rangeStr)
			if len(filtered) != tt.expected {
				t.Errorf("Expected %d chapters, got %d", tt.expected, len(filtered))
			}
		})
	}
}

func TestControllerDownloadManga(t *testing.T) {
	t.Run("successful download setup", func(t *testing.T) {
		controller := &MangaController{
			source: &mockSource{
				getChaptersFunc: func(manga *data.Manga) ([]*data.Chapter, error) {
					return []*data.Chapter{
						{ID: "ch1", Number: "1", Language: "en"},
					}, nil
				},
			},
			repo: &mockRepository{
				saveMangaFunc: func(manga *data.Manga) error {
					return nil
				},
			},
			downloadDir: t.TempDir(),
		}
		
		// Initialize downloader properly
		controller.downloader = NewDownloader(controller.source, controller.repo, controller.downloadDir)
		defer controller.Close()
		
		manga := &data.Manga{ID: "manga-1", Name: "Test"}
		options := DownloadOptions{Language: "ja"} // No Japanese chapters, so should fail gracefully
		
		err := controller.DownloadManga(manga, options)
		if err == nil {
			t.Error("DownloadManga() should fail when no chapters match filters")
		}
	})
	
	t.Run("nil manga", func(t *testing.T) {
		controller := &MangaController{
			source: &mockSource{},
		}
		err := controller.DownloadManga(nil, DownloadOptions{})
		if err == nil {
			t.Error("DownloadManga() should fail with nil manga")
		}
	})
	
	t.Run("no chapters after filtering", func(t *testing.T) {
		controller := &MangaController{
			source: &mockSource{
				getChaptersFunc: func(manga *data.Manga) ([]*data.Chapter, error) {
					return []*data.Chapter{
						{ID: "ch1", Number: "1", Language: "en"},
					}, nil
				},
			},
		}
		
		manga := &data.Manga{ID: "manga-1"}
		options := DownloadOptions{Language: "ja"} // No Japanese chapters
		
		err := controller.DownloadManga(manga, options)
		if err == nil {
			t.Error("DownloadManga() should fail when no chapters match filters")
		}
	})
}

func TestControllerDownloadChapter(t *testing.T) {
	controller := &MangaController{
		downloader: NewDownloader(&mockSource{}, &mockRepository{}, t.TempDir()),
	}
	defer controller.Close()
	
	t.Run("nil manga", func(t *testing.T) {
		chapter := &data.Chapter{ID: "ch1"}
		err := controller.DownloadChapter(nil, chapter)
		if err == nil {
			t.Error("DownloadChapter() should fail with nil manga")
		}
	})
	
	t.Run("nil chapter", func(t *testing.T) {
		manga := &data.Manga{ID: "manga-1"}
		err := controller.DownloadChapter(manga, nil)
		if err == nil {
			t.Error("DownloadChapter() should fail with nil chapter")
		}
	})
}

func TestControllerGetProgressChannel(t *testing.T) {
	controller := &MangaController{
		downloader: NewDownloader(&mockSource{}, &mockRepository{}, t.TempDir()),
	}
	defer controller.Close()
	
	ch := controller.GetProgressChannel()
	if ch == nil {
		t.Error("GetProgressChannel() should not return nil")
	}
}

func TestControllerGetDownloadDirectory(t *testing.T) {
	expectedDir := filepath.Join(t.TempDir(), "downloads")
	controller := &MangaController{
		downloadDir: expectedDir,
	}
	
	got := controller.GetDownloadDirectory()
	if got != expectedDir {
		t.Errorf("Expected directory %s, got %s", expectedDir, got)
	}
}

func TestControllerClose(t *testing.T) {
	controller := NewMangaController()
	
	err := controller.Close()
	if err != nil {
		t.Errorf("Close() error = %v, want nil", err)
	}
	
	// Verify progress channel is closed
	_, ok := <-controller.GetProgressChannel()
	if ok {
		t.Error("Progress channel should be closed after Close()")
	}
}

func TestControllerSaveManga(t *testing.T) {
	savedManga := false
	controller := &MangaController{
		repo: &mockRepository{
			saveMangaFunc: func(manga *data.Manga) error {
				savedManga = true
				return nil
			},
		},
	}
	
	t.Run("successful save", func(t *testing.T) {
		manga := &data.Manga{ID: "manga-1", Name: "Test"}
		err := controller.SaveManga(manga)
		if err != nil {
			t.Errorf("SaveManga() error = %v, want nil", err)
		}
		if !savedManga {
			t.Error("Manga should have been saved")
		}
	})
	
	t.Run("nil manga", func(t *testing.T) {
		err := controller.SaveManga(nil)
		if err == nil {
			t.Error("SaveManga() should fail with nil manga")
		}
	})
}

func TestControllerSaveChapter(t *testing.T) {
	savedChapter := false
	controller := &MangaController{
		repo: &mockRepository{
			saveChapterFunc: func(chapter *data.Chapter) error {
				savedChapter = true
				return nil
			},
		},
	}
	
	t.Run("successful save", func(t *testing.T) {
		chapter := &data.Chapter{ID: "ch1", Number: "1"}
		err := controller.SaveChapter(chapter)
		if err != nil {
			t.Errorf("SaveChapter() error = %v, want nil", err)
		}
		if !savedChapter {
			t.Error("Chapter should have been saved")
		}
	})
	
	t.Run("nil chapter", func(t *testing.T) {
		err := controller.SaveChapter(nil)
		if err == nil {
			t.Error("SaveChapter() should fail with nil chapter")
		}
	})
}

func TestControllerUpdateChapterStatus(t *testing.T) {
	updatedChapter := false
	controller := &MangaController{
		repo: &mockRepository{
			updateChapterStatusFunc: func(chapterID string, downloaded bool, filePath string) error {
				updatedChapter = true
				return nil
			},
		},
	}
	
	t.Run("successful update", func(t *testing.T) {
		err := controller.UpdateChapterStatus("ch1", true, "/path/to/file")
		if err != nil {
			t.Errorf("UpdateChapterStatus() error = %v, want nil", err)
		}
		if !updatedChapter {
			t.Error("Chapter status should have been updated")
		}
	})
	
	t.Run("empty chapter ID", func(t *testing.T) {
		err := controller.UpdateChapterStatus("", true, "/path")
		if err == nil {
			t.Error("UpdateChapterStatus() should fail with empty chapter ID")
		}
	})
}
