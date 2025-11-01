package data

import (
	"database/sql"
	"fmt"
	"log"
	"os"
	"path/filepath"

	_ "github.com/marcboeker/go-duckdb/v2"
)

func InitDuckDB(path string) (*sql.DB, error) {
	// Ensure directory exists
	dir := filepath.Dir(path)
	if err := os.MkdirAll(dir, 0755); err != nil {
		return nil, err
	}

	db, err := sql.Open("duckdb", path)
	if err != nil {
		return nil, err
	}

	// Create tables
	if err := createTables(db); err != nil {
		return nil, err
	}

	return db, nil
}

func createTables(db *sql.DB) error {
	queries := []string{
		`CREATE TABLE IF NOT EXISTS mangas (
			id VARCHAR PRIMARY KEY,
			name VARCHAR NOT NULL,
			description TEXT,
			cover_url VARCHAR,
			source VARCHAR NOT NULL,
			status VARCHAR DEFAULT ''
		)`,
		`CREATE TABLE IF NOT EXISTS chapters (
			id VARCHAR PRIMARY KEY,
			manga_id VARCHAR NOT NULL,
			title VARCHAR,
			language VARCHAR,
			volume VARCHAR,
			number VARCHAR,
			downloaded BOOLEAN DEFAULT false,
			file_path VARCHAR
		)`,
		`CREATE INDEX IF NOT EXISTS idx_chapters_manga_id ON chapters(manga_id)`,
	}

	for _, query := range queries {
		if _, err := db.Exec(query); err != nil {
			return fmt.Errorf("failed to create table: %w", err)
		}
	}

	return nil
}

type Repository struct {
	db *sql.DB
}

var duckDB *sql.DB

func NewDuckDBRepository() *Repository {
	if duckDB == nil {
		homeDir, err := os.UserHomeDir()
		if err != nil {
			log.Fatal(err)
		}
		dbPath := filepath.Join(homeDir, ".mangas", "mangas.db")

		db, err := InitDuckDB(dbPath)
		if err != nil {
			log.Fatal(err)
		}
		duckDB = db
	}

	return &Repository{db: duckDB}
}

// SaveManga inserts or updates a manga in the database
func (r *Repository) SaveManga(manga *Manga) error {
	query := `INSERT INTO mangas (id, name, description, cover_url, source, status)
		VALUES (?, ?, ?, ?, ?, ?)
		ON CONFLICT (id) DO UPDATE SET
			name = excluded.name,
			description = excluded.description,
			cover_url = excluded.cover_url,
			status = excluded.status`

	_, err := r.db.Exec(query, manga.ID, manga.Name, manga.Description, manga.CoverURL, manga.Source, manga.Status)
	return err
}

// GetManga retrieves a manga by ID
func (r *Repository) GetManga(id string) (*Manga, error) {
	query := `SELECT id, name, description, cover_url, source, status FROM mangas WHERE id = ?`

	manga := &Manga{}
	err := r.db.QueryRow(query, id).Scan(
		&manga.ID,
		&manga.Name,
		&manga.Description,
		&manga.CoverURL,
		&manga.Source,
		&manga.Status,
	)

	if err == sql.ErrNoRows {
		return nil, nil
	}
	if err != nil {
		return nil, err
	}

	return manga, nil
}

// ListMangas retrieves all mangas from the database
func (r *Repository) ListMangas() ([]*Manga, error) {
	query := `SELECT id, name, description, cover_url, source, status FROM mangas ORDER BY name`

	rows, err := r.db.Query(query)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var mangas []*Manga
	for rows.Next() {
		manga := &Manga{}
		if err := rows.Scan(
			&manga.ID,
			&manga.Name,
			&manga.Description,
			&manga.CoverURL,
			&manga.Source,
			&manga.Status,
		); err != nil {
			return nil, err
		}
		mangas = append(mangas, manga)
	}

	return mangas, rows.Err()
}

// SaveChapter inserts or updates a chapter in the database
func (r *Repository) SaveChapter(chapter *Chapter) error {
	query := `INSERT INTO chapters (id, manga_id, title, language, volume, number, downloaded, file_path)
		VALUES (?, ?, ?, ?, ?, ?, ?, ?)
		ON CONFLICT (id) DO UPDATE SET
			title = excluded.title,
			language = excluded.language,
			volume = excluded.volume,
			number = excluded.number,
			downloaded = excluded.downloaded,
			file_path = excluded.file_path`

	_, err := r.db.Exec(query,
		chapter.ID,
		chapter.MangaID,
		chapter.Title,
		chapter.Language,
		chapter.Volume,
		chapter.Number,
		chapter.Downloaded,
		chapter.FilePath,
	)
	return err
}

// GetChapters retrieves all chapters for a manga
func (r *Repository) GetChapters(mangaID string) ([]*Chapter, error) {
	query := `SELECT id, manga_id, title, language, volume, number, downloaded, file_path 
		FROM chapters 
		WHERE manga_id = ? 
		ORDER BY CAST(NULLIF(volume, '') AS INTEGER), CAST(NULLIF(number, '') AS DECIMAL)`

	rows, err := r.db.Query(query, mangaID)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var chapters []*Chapter
	for rows.Next() {
		chapter := &Chapter{}
		if err := rows.Scan(
			&chapter.ID,
			&chapter.MangaID,
			&chapter.Title,
			&chapter.Language,
			&chapter.Volume,
			&chapter.Number,
			&chapter.Downloaded,
			&chapter.FilePath,
		); err != nil {
			return nil, err
		}
		chapters = append(chapters, chapter)
	}

	return chapters, rows.Err()
}

// UpdateChapterStatus updates the download status of a chapter
func (r *Repository) UpdateChapterStatus(chapterID string, downloaded bool, filePath string) error {
	query := `UPDATE chapters SET downloaded = ?, file_path = ? WHERE id = ?`
	_, err := r.db.Exec(query, downloaded, filePath, chapterID)
	return err
}

// DeleteManga removes a manga and all its chapters
func (r *Repository) DeleteManga(id string) error {
	// Delete chapters first (no foreign key constraint from chapters to mangas)
	_, err := r.db.Exec(`DELETE FROM chapters WHERE manga_id = ?`, id)
	if err != nil {
		return err
	}

	// Delete manga
	_, err = r.db.Exec(`DELETE FROM mangas WHERE id = ?`, id)
	if err != nil {
		return err
	}

	return nil
}

// GetMangaWithChapterCount retrieves manga with chapter statistics
func (r *Repository) GetMangaWithChapterCount(id string) (*Manga, int, int, error) {
	manga, err := r.GetManga(id)
	if err != nil {
		return nil, 0, 0, err
	}
	if manga == nil {
		return nil, 0, 0, nil
	}

	var total, downloaded int
	query := `SELECT COUNT(*), SUM(CASE WHEN downloaded THEN 1 ELSE 0 END) FROM chapters WHERE manga_id = ?`
	if err := r.db.QueryRow(query, id).Scan(&total, &downloaded); err != nil {
		return manga, 0, 0, err
	}

	return manga, total, downloaded, nil
}
