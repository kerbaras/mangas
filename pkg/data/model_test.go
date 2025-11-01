package data

import "testing"

func TestMangaModel(t *testing.T) {
	manga := Manga{
		ID:          "test-id",
		Name:        "Test Manga",
		Description: "A test manga",
		CoverURL:    "https://example.com/cover.jpg",
		Source:      "mangadex",
		Status:      "completed",
	}

	if manga.ID != "test-id" {
		t.Errorf("Expected ID 'test-id', got '%s'", manga.ID)
	}

	if manga.Name != "Test Manga" {
		t.Errorf("Expected Name 'Test Manga', got '%s'", manga.Name)
	}

	if manga.Status != "completed" {
		t.Errorf("Expected Status 'completed', got '%s'", manga.Status)
	}
}

func TestChapterModel(t *testing.T) {
	chapter := Chapter{
		ID:         "ch-1",
		MangaID:    "manga-1",
		Title:      "Chapter 1",
		Language:   "en",
		Volume:     "1",
		Number:     "1",
		Downloaded: true,
		FilePath:   "/path/to/chapter",
	}

	if chapter.ID != "ch-1" {
		t.Errorf("Expected ID 'ch-1', got '%s'", chapter.ID)
	}

	if chapter.MangaID != "manga-1" {
		t.Errorf("Expected MangaID 'manga-1', got '%s'", chapter.MangaID)
	}

	if !chapter.Downloaded {
		t.Error("Expected Downloaded to be true")
	}

	if chapter.Language != "en" {
		t.Errorf("Expected Language 'en', got '%s'", chapter.Language)
	}
}
