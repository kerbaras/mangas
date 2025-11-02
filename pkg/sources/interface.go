package sources

import (
	"github.com/kerbaras/mangas/pkg/data"
)

type Source interface {
	Search(query string) ([]*data.Manga, error)
	GetManga(id string) (*data.Manga, error)
	GetChapters(manga *data.Manga) ([]*data.Chapter, error)
	GetPages(manga *data.Manga, chapter *data.Chapter) ([]string, error)
	GetMangaCoverURL(manga *data.Manga) (string, error)
	GetChapterCoverURL(manga *data.Manga, chapter *data.Chapter) (string, error)
}
