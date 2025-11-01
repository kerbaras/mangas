package sources

import (
	"encoding/json"
	"fmt"
	"net/http"

	"github.com/kerbaras/mangas/pkg/data"
)

type Manga struct {
	ID         string `json:"id"`
	Attributes struct {
		Title       map[string]string `json:"title"`
		Description map[string]string `json:"description"`
	} `json:"attributes"`
}

func (m *Manga) ToManga() *data.Manga {
	return &data.Manga{
		ID:   m.ID,
		Name: m.Attributes.Title["en"],
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
		ID:       c.ID,
		Title:    c.Attributes.Title,
		Language: c.Attributes.Language,
		Volume:   c.Attributes.Volume,
		Number:   c.Attributes.Number,
	}
}

type MangaDex struct {
	api     *http.Client
	baseURL string
}

func NewMangaDex() *MangaDex {
	api := http.DefaultClient
	baseURL := "https://api.mangadex.org"
	return &MangaDex{api: api, baseURL: baseURL}
}

func (m *MangaDex) get(url string, v any) error {
	url = fmt.Sprintf("%s%s", m.baseURL, url)
	req, err := http.NewRequest("GET", url, nil)
	if err != nil {
		return err
	}
	req.Header.Set("Accept", "application/json")
	req.Header.Set("Content-Type", "application/json")
	resp, err := m.api.Do(req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()
	return json.NewDecoder(resp.Body).Decode(v)
}

func (m *MangaDex) Search(query string) ([]data.Manga, error) {
	url := fmt.Sprintf("/manga?title=%s", query)
	var mangas struct {
		Data []Manga `json:"data"`
	}
	if err := m.get(url, &mangas); err != nil {
		return nil, err
	}
	out := make([]data.Manga, len(mangas.Data))
	for i, manga := range mangas.Data {
		out[i] = *manga.ToManga()
	}
	return out, nil
}

func (m *MangaDex) GetManga(id string) (*data.Manga, error) {
	url := fmt.Sprintf("/manga/%s", id)
	var manga struct {
		Data Manga `json:"data"`
	}
	if err := m.get(url, &manga); err != nil {
		return nil, err
	}
	return manga.Data.ToManga(), nil
}

func (m *MangaDex) GetChapters(manga *data.Manga) ([]data.Chapter, error) {
	url := fmt.Sprintf("/manga/%s/feed", manga.ID)
	var feed struct {
		Data []Chapter `json:"data"`
	}
	if err := m.get(url, &feed); err != nil {
		return nil, err
	}
	out := make([]data.Chapter, len(feed.Data))
	for i, chapter := range feed.Data {
		out[i] = *chapter.ToChapter()
	}
	return out, nil
}

func (m *MangaDex) GetPages(_ *data.Manga, chapter *data.Chapter) ([]string, error) {
	url := fmt.Sprintf("/at-home/server/%s", chapter.ID)
	var server struct {
		BaseURL string `json:"baseUrl"`
		Chapter struct {
			Hash string   `json:"hash"`
			Data []string `json:"data"`
		} `json:"chapter"`
	}
	if err := m.get(url, &server); err != nil {
		return nil, err
	}
	pages := make([]string, len(server.Chapter.Data))
	for i, data := range server.Chapter.Data {
		pages[i] = fmt.Sprintf("%s/data/%s/%s", server.BaseURL, server.Chapter.Hash, data)
	}
	return pages, nil
}
