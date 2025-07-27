package data

import (
	"database/sql"
	"log"

	_ "github.com/marcboeker/go-duckdb/v2"
)

func InitDuckDB(path string) (*sql.DB, error) {
	db, err := sql.Open("duckdb", path)
	if err != nil {
		return nil, err
	}

	return db, err
}

type Repository struct {
	db *sql.DB
}

var duckDB *sql.DB

func NewDuckDBRepository() *Repository {
	if duckDB == nil {
		db, err := InitDuckDB("mangas.db")
		if err != nil {
			log.Fatal(err)
		}
		duckDB = db
	}

	return &Repository{db: duckDB}
}
