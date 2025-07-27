package sources

import (
	"github.com/kerbaras/mangas/pkg/data"
	"github.com/kerbaras/mangas/pkg/integrations"
)

type Source interface {
	Search(query string) ([]data.Manga, error)
	GetManga(id string) (data.Manga, error)
	GetChapters(mangaID string) ([]data.Chapter, error)
	GetChapter(mangaID, chapterID string) (data.Chapter, error)

	Download(chapter data.Chapter, processor integrations.Processor) error
}
