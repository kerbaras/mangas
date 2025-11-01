package sources

import (
	"testing"

	"github.com/kerbaras/mangas/pkg/data"
	"github.com/stretchr/testify/assert"
)

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
