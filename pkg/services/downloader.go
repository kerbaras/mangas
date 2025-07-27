package services

import (
	"github.com/kerbaras/mangas/pkg/data"
	"github.com/kerbaras/mangas/pkg/sources"
)

type Downloader struct {
	source sources.Source
}

func NewDownloader(source sources.Source) *Downloader {
	return &Downloader{source: source}
}

func (d *Downloader) Download(chapter data.Chapter) error {
	return nil
}
