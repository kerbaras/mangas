package components

import (
	"strings"
	"testing"

	"github.com/kerbaras/mangas/pkg/data"
)

func TestNewMangaList(t *testing.T) {
	list := NewMangaList()

	if list == nil {
		t.Fatal("Expected manga list to be created")
	}

	if list.SelectedIndex != 0 {
		t.Errorf("Expected SelectedIndex 0, got %d", list.SelectedIndex)
	}

	if len(list.Items) != 0 {
		t.Errorf("Expected 0 items, got %d", len(list.Items))
	}
}

func TestSetItems(t *testing.T) {
	list := NewMangaList()

	items := []MangaListItem{
		{Manga: &data.Manga{ID: "1", Name: "Manga 1"}},
		{Manga: &data.Manga{ID: "2", Name: "Manga 2"}},
	}

	list.SetItems(items)

	if len(list.Items) != 2 {
		t.Errorf("Expected 2 items, got %d", len(list.Items))
	}

	if list.SelectedIndex != 0 {
		t.Errorf("Expected SelectedIndex 0, got %d", list.SelectedIndex)
	}
}

func TestSetItemsResetsSelection(t *testing.T) {
	list := NewMangaList()

	items := []MangaListItem{
		{Manga: &data.Manga{ID: "1", Name: "Manga 1"}},
		{Manga: &data.Manga{ID: "2", Name: "Manga 2"}},
		{Manga: &data.Manga{ID: "3", Name: "Manga 3"}},
	}

	list.SetItems(items)
	list.SelectedIndex = 2

	// Set fewer items
	newItems := []MangaListItem{
		{Manga: &data.Manga{ID: "1", Name: "Manga 1"}},
	}

	list.SetItems(newItems)

	if list.SelectedIndex != 0 {
		t.Errorf("Expected SelectedIndex to be reset to 0, got %d", list.SelectedIndex)
	}
}

func TestNext(t *testing.T) {
	list := NewMangaList()

	items := []MangaListItem{
		{Manga: &data.Manga{ID: "1", Name: "Manga 1"}},
		{Manga: &data.Manga{ID: "2", Name: "Manga 2"}},
		{Manga: &data.Manga{ID: "3", Name: "Manga 3"}},
	}

	list.SetItems(items)

	// Move next
	list.Next()
	if list.SelectedIndex != 1 {
		t.Errorf("Expected SelectedIndex 1, got %d", list.SelectedIndex)
	}

	list.Next()
	if list.SelectedIndex != 2 {
		t.Errorf("Expected SelectedIndex 2, got %d", list.SelectedIndex)
	}

	// Should wrap around
	list.Next()
	if list.SelectedIndex != 0 {
		t.Errorf("Expected SelectedIndex to wrap to 0, got %d", list.SelectedIndex)
	}
}

func TestPrev(t *testing.T) {
	list := NewMangaList()

	items := []MangaListItem{
		{Manga: &data.Manga{ID: "1", Name: "Manga 1"}},
		{Manga: &data.Manga{ID: "2", Name: "Manga 2"}},
		{Manga: &data.Manga{ID: "3", Name: "Manga 3"}},
	}

	list.SetItems(items)

	// Should wrap around when at start
	list.Prev()
	if list.SelectedIndex != 2 {
		t.Errorf("Expected SelectedIndex to wrap to 2, got %d", list.SelectedIndex)
	}

	list.Prev()
	if list.SelectedIndex != 1 {
		t.Errorf("Expected SelectedIndex 1, got %d", list.SelectedIndex)
	}

	list.Prev()
	if list.SelectedIndex != 0 {
		t.Errorf("Expected SelectedIndex 0, got %d", list.SelectedIndex)
	}
}

func TestNextPrevEmptyList(t *testing.T) {
	list := NewMangaList()

	// Should not panic with empty list
	list.Next()
	list.Prev()

	if list.SelectedIndex != 0 {
		t.Errorf("Expected SelectedIndex to remain 0, got %d", list.SelectedIndex)
	}
}

func TestSelected(t *testing.T) {
	list := NewMangaList()

	// Empty list
	if list.Selected() != nil {
		t.Error("Expected nil for empty list")
	}

	items := []MangaListItem{
		{Manga: &data.Manga{ID: "1", Name: "Manga 1"}},
		{Manga: &data.Manga{ID: "2", Name: "Manga 2"}},
	}

	list.SetItems(items)

	selected := list.Selected()
	if selected == nil {
		t.Fatal("Expected selected item")
	}

	if selected.Manga.ID != "1" {
		t.Errorf("Expected selected manga ID '1', got '%s'", selected.Manga.ID)
	}

	list.Next()
	selected = list.Selected()
	if selected.Manga.ID != "2" {
		t.Errorf("Expected selected manga ID '2', got '%s'", selected.Manga.ID)
	}
}

func TestViewEmptyList(t *testing.T) {
	list := NewMangaList()
	list.Width = 80
	list.Height = 20

	view := list.View()

	if !strings.Contains(view, "No manga in library") {
		t.Error("Expected 'No manga in library' message")
	}
}

func TestViewWithItems(t *testing.T) {
	list := NewMangaList()
	list.Width = 80
	list.Height = 20

	items := []MangaListItem{
		{
			Manga:           &data.Manga{ID: "1", Name: "Test Manga", Status: "completed"},
			ChapterCount:    10,
			DownloadedCount: 5,
		},
	}

	list.SetItems(items)

	view := list.View()

	if !strings.Contains(view, "Test Manga") {
		t.Error("Expected manga name in view")
	}

	if !strings.Contains(view, "5 / 10 downloaded") {
		t.Error("Expected chapter count in view")
	}
}

func TestMangaListItem(t *testing.T) {
	manga := &data.Manga{
		ID:     "test-id",
		Name:   "Test Manga",
		Status: "downloading",
	}

	item := MangaListItem{
		Manga:           manga,
		ChapterCount:    20,
		DownloadedCount: 10,
	}

	if item.Manga.ID != "test-id" {
		t.Errorf("Expected manga ID 'test-id', got '%s'", item.Manga.ID)
	}

	if item.ChapterCount != 20 {
		t.Errorf("Expected ChapterCount 20, got %d", item.ChapterCount)
	}

	if item.DownloadedCount != 10 {
		t.Errorf("Expected DownloadedCount 10, got %d", item.DownloadedCount)
	}
}

