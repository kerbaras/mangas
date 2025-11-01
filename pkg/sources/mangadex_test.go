package sources

import (
	"testing"

	"github.com/kerbaras/mangas/pkg/data"
	"github.com/stretchr/testify/assert"
)

func TestMangaToManga(t *testing.T) {
	mdManga := &Manga{
		ID: "test-id",
		Attributes: struct {
			Title       map[string]string `json:"title"`
			Description map[string]string `json:"description"`
		}{
			Title: map[string]string{
				"en": "English Title",
				"ja": "日本語タイトル",
			},
			Description: map[string]string{
				"en": "English Description",
			},
		},
	}

	manga := mdManga.ToManga()

	assert.Equal(t, manga.ID, "test-id")
	assert.Equal(t, manga.Name, "English Title")
	assert.Equal(t, manga.Description, "English Description")
	assert.Equal(t, manga.Source, "mangadex")
}

func TestMangaToMangaFallback(t *testing.T) {
	// Test fallback when English title is not available
	mdManga := &Manga{
		ID: "test-id",
		Attributes: struct {
			Title       map[string]string `json:"title"`
			Description map[string]string `json:"description"`
		}{
			Title: map[string]string{
				"ja": "日本語タイトル",
			},
			Description: map[string]string{
				"ja": "日本語の説明",
			},
		},
	}

	manga := mdManga.ToManga()

	assert.Equal(t, manga.Name, "日本語タイトル")
	assert.Equal(t, manga.Description, "日本語の説明")
}

func TestChapterToChapter(t *testing.T) {
	mdChapter := &Chapter{
		ID: "chapter-id",
		Attributes: struct {
			Title    string   `json:"title"`
			Language string   `json:"translatedLanguage"`
			Hash     string   `json:"hash"`
			Data     []string `json:"data"`
			MangaID  string   `json:"mangaId"`
			Volume   string   `json:"volume"`
			Number   string   `json:"chapter"`
		}{
			Title:    "Test Chapter",
			Language: "en",
			Volume:   "1",
			Number:   "5",
		},
	}

	chapter := mdChapter.ToChapter()

	assert.Equal(t, chapter.ID, "chapter-id")
	assert.Equal(t, chapter.Title, "Test Chapter")
	assert.Equal(t, chapter.Language, "en")
	assert.Equal(t, chapter.Volume, "1")
	assert.Equal(t, chapter.Number, "5")
	assert.False(t, chapter.Downloaded)
	assert.Empty(t, chapter.FilePath)

	if chapter.Downloaded {
		assert.False(t, chapter.Downloaded)
		assert.Empty(t, chapter.FilePath)
	}
}

// Test interface implementation
func TestMangaDex_ImplementsSource(t *testing.T) {
	md := NewMangaDex()
	assert.Implements(t, new(Source), md)
}

func TestSourceInterfaceMethods(t *testing.T) {
	md := NewMangaDex()
	assert.NotPanics(t, func() {
		md.Search("test")
	})
	assert.NotPanics(t, func() {
		md.GetManga("test-id")
	})
	assert.NotPanics(t, func() {
		md.GetChapters(&data.Manga{ID: "test-id"})
	})
	assert.NotPanics(t, func() {
		md.GetPages(&data.Manga{}, &data.Chapter{ID: "test-id"})
	})
}

func TestMangaDex_Search(t *testing.T) {
	md := NewMangaDex()
	mangas, err := md.Search("naruto")
	if err != nil {
		t.Fatalf("Search failed: %v", err)
	}
	assert.NoError(t, err)
	// check if the number of mangas is greater than 0
	assert.Greater(t, len(mangas), 0)
	// check if the first manga is not empty
	assert.NotEmpty(t, mangas[0].ID)
	assert.NotEmpty(t, mangas[0].Name)
	// check if the first manga is Naruto
	assert.Contains(t, mangas[0].Name, "Naruto")
}

func TestMangaDex_GetManga(t *testing.T) {
	md := NewMangaDex()
	manga, err := md.GetManga("6b1eb93e-473a-4ab3-9922-1a66d2a29a4a")
	assert.NoError(t, err)
	assert.Equal(t, manga.ID, "6b1eb93e-473a-4ab3-9922-1a66d2a29a4a")
	assert.Equal(t, manga.Name, "Naruto")
}

func TestMangaDex_GetChapters(t *testing.T) {
	md := NewMangaDex()
	manga := &data.Manga{
		ID:   "6b1eb93e-473a-4ab3-9922-1a66d2a29a4a",
		Name: "Naruto",
	}
	chapters, err := md.GetChapters(manga)
	assert.NoError(t, err)
	assert.Greater(t, len(chapters), 0)
	assert.Equal(t, chapters[0].ID, "cd5635a9-5e2d-41ef-9fe1-2ff13cdf5841")
	assert.Equal(t, chapters[0].Title, "Uzumaki Naruto!")
	assert.Equal(t, chapters[0].Language, "en")
	assert.Equal(t, chapters[0].Volume, "1")
	assert.Equal(t, chapters[0].Number, "1")
}

func TestMangaDex_GetPages(t *testing.T) {
	md := NewMangaDex()
	manga := &data.Manga{
		ID:   "be282a1e-5a13-4f89-9d98-7da56d5dbb1e",
		Name: "A Mistress Who Tempts Her Maid",
	}
	chapter := &data.Chapter{
		ID: "a54c491c-8e4c-4e97-8873-5b79e59da210",
	}
	pages, err := md.GetPages(manga, chapter)
	assert.NoError(t, err)
	assert.Len(t, pages, 6)
}
