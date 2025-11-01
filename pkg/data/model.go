package data

type Manga struct {
	ID          string
	Name        string
	Description string
	CoverURL    string
	Source      string
	Status      string // "downloading", "completed", "error"
}

type Chapter struct {
	ID         string
	MangaID    string
	Title      string
	Language   string
	Volume     string
	Number     string
	Downloaded bool
	FilePath   string // Path to downloaded images directory
}
