package sources

import (
	"fmt"
	"net/url"

	"github.com/kerbaras/mangas/pkg/data"
	"github.com/kerbaras/mangas/pkg/utils"
)

type Manga struct {
	ID           string `json:"id"`
	Attributes   struct {
		Title       map[string]string `json:"title"`
		Description map[string]string `json:"description"`
	} `json:"attributes"`
	Relationships []struct {
		Type       string `json:"type"`
		ID         string `json:"id"`
		Attributes struct {
			FileName string `json:"fileName"`
		} `json:"attributes"`
	} `json:"relationships"`
}

func (m *Manga) ToManga() *data.Manga {
	title := m.Attributes.Title["en"]
	if title == "" {
		// Fallback to first available title
		for _, v := range m.Attributes.Title {
			title = v
			break
		}
	}

	description := m.Attributes.Description["en"]
	if description == "" {
		for _, v := range m.Attributes.Description {
			description = v
			break
		}
	}

	return &data.Manga{
		ID:          m.ID,
		Name:        title,
		Description: description,
		Source:      "mangadex",
		Status:      "",
	}
}

type Chapter struct {
	data.Chapter
	ID         string `json:"id"`
	Attributes struct {
		Title    string   `json:"title"`
		Language string   `json:"translatedLanguage"`
		Hash     string   `json:"hash"`
		Data     []string `json:"data"`
		MangaID  string   `json:"mangaId"`
		Volume   string   `json:"volume"`
		Number   string   `json:"chapter"`
	} `json:"attributes"`
}

func (c *Chapter) ToChapter() *data.Chapter {
	return &data.Chapter{
		ID:         c.ID,
		Title:      c.Attributes.Title,
		Language:   c.Attributes.Language,
		Volume:     c.Attributes.Volume,
		Number:     c.Attributes.Number,
		Downloaded: false,
		FilePath:   "",
	}
}

type MangaDex struct {
	api *utils.API
}

func (m *MangaDex) Search(query string) ([]*data.Manga, error) {
	params := url.Values{
		"title": {query},
		"limit": {"10"},
	}
	var mangas struct {
		Data []Manga `json:"data"`
	}
	if err := m.api.Get("/manga", params, &mangas); err != nil {
		return nil, err
	}
	out := make([]*data.Manga, len(mangas.Data))
	for i, manga := range mangas.Data {
		out[i] = manga.ToManga()
	}
	return out, nil
}

func (m *MangaDex) GetManga(id string) (*data.Manga, error) {
	var manga struct {
		Data Manga `json:"data"`
	}
	if err := m.api.Get(fmt.Sprintf("/manga/%s", id), nil, &manga); err != nil {
		return nil, err
	}
	return manga.Data.ToManga(), nil
}

func (m *MangaDex) GetChapters(manga *data.Manga) ([]*data.Chapter, error) {
	var feed struct {
		Data []Chapter `json:"data"`
	}
	if err := m.api.Get(fmt.Sprintf("/manga/%s/feed", manga.ID), nil, &feed); err != nil {
		return nil, err
	}
	out := make([]*data.Chapter, len(feed.Data))
	for i, chapter := range feed.Data {
		out[i] = chapter.ToChapter()
	}
	return out, nil
}

func (m *MangaDex) GetPages(_ *data.Manga, chapter *data.Chapter) ([]string, error) {
	var server struct {
		BaseURL string `json:"baseUrl"`
		Chapter struct {
			Hash string   `json:"hash"`
			Data []string `json:"data"`
		} `json:"chapter"`
	}
	if err := m.api.Get(fmt.Sprintf("/at-home/server/%s", chapter.ID), nil, &server); err != nil {
		return nil, err
	}
	pages := make([]string, len(server.Chapter.Data))
	for i, data := range server.Chapter.Data {
		pages[i] = fmt.Sprintf("%s/data/%s/%s", server.BaseURL, server.Chapter.Hash, data)
	}
	return pages, nil
}

// GetMangaCoverURL returns the cover image URL for a manga
func (m *MangaDex) GetMangaCoverURL(manga *data.Manga) (string, error) {
	// Get manga with relationships to find cover art
	var mangaResp struct {
		Data Manga `json:"data"`
	}
	params := url.Values{
		"includes[]": {"cover_art"},
	}
	if err := m.api.Get(fmt.Sprintf("/manga/%s", manga.ID), params, &mangaResp); err != nil {
		return "", err
	}

	// Find cover art relationship
	var coverID, coverFileName string
	for _, rel := range mangaResp.Data.Relationships {
		if rel.Type == "cover_art" {
			coverID = rel.ID
			coverFileName = rel.Attributes.FileName
			break
		}
	}

	if coverID == "" || coverFileName == "" {
		return "", fmt.Errorf("no cover art found for manga")
	}

	// Construct cover URL
	// MangaDex cover URLs: https://uploads.mangadex.org/covers/{manga-id}/{filename}
	coverURL := fmt.Sprintf("https://uploads.mangadex.org/covers/%s/%s", manga.ID, coverFileName)
	return coverURL, nil
}

// GetChapterCoverURL returns the cover image URL for a chapter
// Note: MangaDex doesn't typically have separate chapter covers, so we return the manga cover
// or the first page of the chapter as a fallback
func (m *MangaDex) GetChapterCoverURL(manga *data.Manga, chapter *data.Chapter) (string, error) {
	// For MangaDex, chapters don't have separate covers
	// We can either return the manga cover or the first page
	// Let's return the manga cover for consistency
	return m.GetMangaCoverURL(manga)
}

func NewMangaDex() Source {
	baseURL := "https://api.mangadex.org"
	return &MangaDex{api: utils.NewAPI(baseURL)}
}
