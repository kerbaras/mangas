package data

import (
	"os"
	"path/filepath"
	"testing"
)

func setupTestDB(t *testing.T) (*Repository, func()) {
	t.Helper()
	
	// Create temp directory for test database
	tmpDir, err := os.MkdirTemp("", "mangas-test-*")
	if err != nil {
		t.Fatalf("Failed to create temp dir: %v", err)
	}

	dbPath := filepath.Join(tmpDir, "test.db")
	db, err := InitDuckDB(dbPath)
	if err != nil {
		os.RemoveAll(tmpDir)
		t.Fatalf("Failed to init DB: %v", err)
	}

	repo := &Repository{db: db}

	cleanup := func() {
		db.Close()
		os.RemoveAll(tmpDir)
	}

	return repo, cleanup
}

func TestSaveAndGetManga(t *testing.T) {
	repo, cleanup := setupTestDB(t)
	defer cleanup()

	manga := &Manga{
		ID:          "test-manga-1",
		Name:        "Test Manga",
		Description: "A test manga description",
		CoverURL:    "https://example.com/cover.jpg",
		Source:      "mangadex",
		Status:      "completed",
	}

	// Save manga
	err := repo.SaveManga(manga)
	if err != nil {
		t.Fatalf("Failed to save manga: %v", err)
	}

	// Get manga
	retrieved, err := repo.GetManga("test-manga-1")
	if err != nil {
		t.Fatalf("Failed to get manga: %v", err)
	}

	if retrieved == nil {
		t.Fatal("Expected manga to be found")
	}

	if retrieved.ID != manga.ID {
		t.Errorf("Expected ID %s, got %s", manga.ID, retrieved.ID)
	}

	if retrieved.Name != manga.Name {
		t.Errorf("Expected Name %s, got %s", manga.Name, retrieved.Name)
	}

	if retrieved.Status != manga.Status {
		t.Errorf("Expected Status %s, got %s", manga.Status, retrieved.Status)
	}
}

func TestListMangas(t *testing.T) {
	repo, cleanup := setupTestDB(t)
	defer cleanup()

	// Initially empty
	mangas, err := repo.ListMangas()
	if err != nil {
		t.Fatalf("Failed to list mangas: %v", err)
	}

	if len(mangas) != 0 {
		t.Errorf("Expected 0 mangas, got %d", len(mangas))
	}

	// Add some mangas
	for i := 1; i <= 3; i++ {
		manga := &Manga{
			ID:     string(rune('a' + i - 1)),
			Name:   string(rune('A' + i - 1)) + " Manga",
			Source: "mangadex",
		}
		err := repo.SaveManga(manga)
		if err != nil {
			t.Fatalf("Failed to save manga %d: %v", i, err)
		}
	}

	// List all
	mangas, err = repo.ListMangas()
	if err != nil {
		t.Fatalf("Failed to list mangas: %v", err)
	}

	if len(mangas) != 3 {
		t.Errorf("Expected 3 mangas, got %d", len(mangas))
	}
}

func TestSaveAndGetChapters(t *testing.T) {
	repo, cleanup := setupTestDB(t)
	defer cleanup()

	// First save a manga
	manga := &Manga{
		ID:     "manga-1",
		Name:   "Test Manga",
		Source: "mangadex",
	}
	repo.SaveManga(manga)

	// Save chapters
	chapters := []*Chapter{
		{
			ID:       "ch-1",
			MangaID:  "manga-1",
			Title:    "Chapter 1",
			Language: "en",
			Volume:   "1",
			Number:   "1",
		},
		{
			ID:       "ch-2",
			MangaID:  "manga-1",
			Title:    "Chapter 2",
			Language: "en",
			Volume:   "1",
			Number:   "2",
		},
	}

	for _, ch := range chapters {
		err := repo.SaveChapter(ch)
		if err != nil {
			t.Fatalf("Failed to save chapter: %v", err)
		}
	}

	// Get chapters
	retrieved, err := repo.GetChapters("manga-1")
	if err != nil {
		t.Fatalf("Failed to get chapters: %v", err)
	}

	if len(retrieved) != 2 {
		t.Errorf("Expected 2 chapters, got %d", len(retrieved))
	}

	// Verify ordering (should be by volume, then number)
	if len(retrieved) >= 2 {
		if retrieved[0].Number != "1" {
			t.Errorf("Expected first chapter number '1', got '%s'", retrieved[0].Number)
		}
		if retrieved[1].Number != "2" {
			t.Errorf("Expected second chapter number '2', got '%s'", retrieved[1].Number)
		}
	}
}

func TestUpdateChapterStatus(t *testing.T) {
	repo, cleanup := setupTestDB(t)
	defer cleanup()

	manga := &Manga{ID: "manga-1", Name: "Test", Source: "test"}
	repo.SaveManga(manga)

	chapter := &Chapter{
		ID:         "ch-1",
		MangaID:    "manga-1",
		Number:     "1",
		Volume:     "1",
		Language:   "en",
		Downloaded: false,
	}
	err := repo.SaveChapter(chapter)
	if err != nil {
		t.Fatalf("Failed to save chapter: %v", err)
	}

	// Update status
	err = repo.UpdateChapterStatus("ch-1", true, "/path/to/chapter")
	if err != nil {
		t.Fatalf("Failed to update chapter status: %v", err)
	}

	// Verify
	chapters, err := repo.GetChapters("manga-1")
	if err != nil {
		t.Fatalf("Failed to get chapters: %v", err)
	}
	
	if len(chapters) == 0 {
		t.Fatal("No chapters found")
	}

	if !chapters[0].Downloaded {
		t.Error("Expected chapter to be marked as downloaded")
	}

	if chapters[0].FilePath != "/path/to/chapter" {
		t.Errorf("Expected FilePath '/path/to/chapter', got '%s'", chapters[0].FilePath)
	}
}

func TestDeleteManga(t *testing.T) {
	repo, cleanup := setupTestDB(t)
	defer cleanup()

	manga := &Manga{ID: "manga-1", Name: "Test", Source: "test"}
	repo.SaveManga(manga)

	chapter := &Chapter{ID: "ch-1", MangaID: "manga-1", Number: "1"}
	repo.SaveChapter(chapter)

	// Delete manga
	err := repo.DeleteManga("manga-1")
	if err != nil {
		t.Fatalf("Failed to delete manga: %v", err)
	}

	// Verify manga is gone
	retrieved, _ := repo.GetManga("manga-1")
	if retrieved != nil {
		t.Error("Expected manga to be deleted")
	}

	// Verify chapters are gone too
	chapters, _ := repo.GetChapters("manga-1")
	if len(chapters) != 0 {
		t.Errorf("Expected 0 chapters, got %d", len(chapters))
	}
}

func TestGetMangaWithChapterCount(t *testing.T) {
	repo, cleanup := setupTestDB(t)
	defer cleanup()

	manga := &Manga{ID: "manga-1", Name: "Test", Source: "test"}
	repo.SaveManga(manga)

	// Add 3 chapters, 2 downloaded
	chapters := []*Chapter{
		{ID: "ch-1", MangaID: "manga-1", Number: "1", Downloaded: true},
		{ID: "ch-2", MangaID: "manga-1", Number: "2", Downloaded: true},
		{ID: "ch-3", MangaID: "manga-1", Number: "3", Downloaded: false},
	}

	for _, ch := range chapters {
		repo.SaveChapter(ch)
	}

	// Get stats
	retrievedManga, total, downloaded, err := repo.GetMangaWithChapterCount("manga-1")
	if err != nil {
		t.Fatalf("Failed to get manga with chapter count: %v", err)
	}

	if retrievedManga == nil {
		t.Fatal("Expected manga to be found")
	}

	if total != 3 {
		t.Errorf("Expected 3 total chapters, got %d", total)
	}

	if downloaded != 2 {
		t.Errorf("Expected 2 downloaded chapters, got %d", downloaded)
	}
}

func TestGetNonExistentManga(t *testing.T) {
	repo, cleanup := setupTestDB(t)
	defer cleanup()

	manga, err := repo.GetManga("non-existent")
	if err != nil {
		t.Fatalf("Expected no error, got: %v", err)
	}

	if manga != nil {
		t.Error("Expected manga to be nil for non-existent ID")
	}
}

func TestSaveMangaUpsert(t *testing.T) {
	repo, cleanup := setupTestDB(t)
	defer cleanup()

	manga := &Manga{
		ID:     "manga-1",
		Name:   "Original Name",
		Source: "test",
		Status: "downloading",
	}
	repo.SaveManga(manga)

	// Update same manga
	manga.Name = "Updated Name"
	manga.Status = "completed"
	err := repo.SaveManga(manga)
	if err != nil {
		t.Fatalf("Failed to update manga: %v", err)
	}

	// Verify update
	retrieved, _ := repo.GetManga("manga-1")
	if retrieved.Name != "Updated Name" {
		t.Errorf("Expected Name 'Updated Name', got '%s'", retrieved.Name)
	}

	if retrieved.Status != "completed" {
		t.Errorf("Expected Status 'completed', got '%s'", retrieved.Status)
	}
}

