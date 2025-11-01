package data

import (
	"os"
	"path/filepath"
	"testing"
)

func TestInitDuckDB(t *testing.T) {
	tmpDir, err := os.MkdirTemp("", "test-init-*")
	if err != nil {
		t.Fatalf("Failed to create temp dir: %v", err)
	}
	defer os.RemoveAll(tmpDir)

	dbPath := filepath.Join(tmpDir, "test.db")
	
	db, err := InitDuckDB(dbPath)
	if err != nil {
		t.Fatalf("Failed to initialize DB: %v", err)
	}
	defer db.Close()

	// Verify tables exist
	var tableCount int
	err = db.QueryRow(`SELECT COUNT(*) FROM information_schema.tables WHERE table_name IN ('mangas', 'chapters')`).Scan(&tableCount)
	if err != nil {
		t.Fatalf("Failed to query tables: %v", err)
	}

	if tableCount != 2 {
		t.Errorf("Expected 2 tables, got %d", tableCount)
	}
}

func TestInitDuckDBCreatesDirectory(t *testing.T) {
	tmpDir, err := os.MkdirTemp("", "test-init-*")
	if err != nil {
		t.Fatalf("Failed to create temp dir: %v", err)
	}
	defer os.RemoveAll(tmpDir)

	// Use nested directory that doesn't exist
	dbPath := filepath.Join(tmpDir, "nested", "dir", "test.db")
	
	db, err := InitDuckDB(dbPath)
	if err != nil {
		t.Fatalf("Failed to initialize DB with nested path: %v", err)
	}
	defer db.Close()

	// Verify file was created
	if _, err := os.Stat(dbPath); os.IsNotExist(err) {
		t.Error("DB file was not created")
	}
}

func TestNewDuckDBRepositorySingleton(t *testing.T) {
	// Reset global var for testing
	oldDB := duckDB
	duckDB = nil
	defer func() { duckDB = oldDB }()

	repo1 := NewDuckDBRepository()
	repo2 := NewDuckDBRepository()

	// Both should reference the same underlying DB
	if repo1.db != repo2.db {
		t.Error("Expected singleton pattern - both repos should share the same DB")
	}
}

