package services

import (
	"os"
	"path/filepath"

	"github.com/kerbaras/mangas/pkg/data"
	"github.com/kerbaras/mangas/pkg/sources"
)

type MangaController struct {
	source     sources.Source
	repo       *data.Repository
	downloader *Downloader
}

func NewMangaController() *MangaController {
	source := sources.NewMangaDex()
	repo := data.NewDuckDBRepository()

	homeDir, _ := os.UserHomeDir()
	downloadDir := filepath.Join(homeDir, "Downloads")

	downloader := NewDownloader(source, repo, downloadDir)
	return &MangaController{source: source, repo: repo, downloader: downloader}
}
