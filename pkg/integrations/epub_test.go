package integrations

import (
	"os"
	"path/filepath"
	"testing"

	"github.com/kerbaras/mangas/pkg/data"
)

func setupTestEPub(t *testing.T) (string, string, func()) {
	t.Helper()

	// Create temp directories
	outputDir, err := os.MkdirTemp("", "epub-output-*")
	if err != nil {
		t.Fatalf("Failed to create output dir: %v", err)
	}

	chapterDir, err := os.MkdirTemp("", "chapter-data-*")
	if err != nil {
		os.RemoveAll(outputDir)
		t.Fatalf("Failed to create chapter dir: %v", err)
	}

	cleanup := func() {
		os.RemoveAll(outputDir)
		os.RemoveAll(chapterDir)
	}

	return outputDir, chapterDir, cleanup
}

func createTestImage(t *testing.T, dir string, filename string) {
	t.Helper()

	// Create a simple 1x1 PNG
	pngData := []byte{
		0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
		0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, // IHDR chunk
		0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, // 1x1 dimensions
		0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
		0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, // IDAT chunk
		0x54, 0x08, 0x99, 0x63, 0xF8, 0x0F, 0x00, 0x00,
		0x01, 0x01, 0x00, 0x05, 0x18, 0x0D, 0xA3, 0xD2,
		0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, // IEND chunk
		0xAE, 0x42, 0x60, 0x82,
	}

	path := filepath.Join(dir, filename)
	if err := os.WriteFile(path, pngData, 0644); err != nil {
		t.Fatalf("Failed to create test image: %v", err)
	}
}

func TestNewEPubBuilder(t *testing.T) {
	processor := NewEPubBuilder()
	if processor == nil {
		t.Fatal("Expected processor to be created")
	}

	if processor.outputDir != "/tmp/test" {
		t.Errorf("Expected outputDir '/tmp/test', got '%s'", processor.outputDir)
	}
}

func TestCreateEPub(t *testing.T) {
	outputDir, chapterDir, cleanup := setupTestEPub(t)
	defer cleanup()

	// Create test images
	createTestImage(t, chapterDir, "0001.png")
	createTestImage(t, chapterDir, "0002.png")

	processor := NewEPubBuilder()

	manga := &data.Manga{
		ID:          "test-manga",
		Name:        "Test Manga",
		Description: "A test manga for EPub generation",
		Source:      "test",
	}

	chapters := []*data.Chapter{
		{
			ID:         "ch-1",
			MangaID:    "test-manga",
			Title:      "First Chapter",
			Volume:     "1",
			Number:     "1",
			Downloaded: true,
			FilePath:   chapterDir,
		},
	}

	epubPath, err := processor.CreateEPub(manga, chapters)
	if err != nil {
		t.Fatalf("Failed to create EPub: %v", err)
	}

	// Verify EPub file exists
	if _, err := os.Stat(epubPath); os.IsNotExist(err) {
		t.Errorf("EPub file was not created at %s", epubPath)
	}

	// Verify it's in the correct directory
	if filepath.Dir(epubPath) != outputDir {
		t.Errorf("Expected EPub in %s, got %s", outputDir, filepath.Dir(epubPath))
	}

	// Verify filename is sanitized
	expectedName := "Test Manga.epub"
	if filepath.Base(epubPath) != expectedName {
		t.Errorf("Expected filename '%s', got '%s'", expectedName, filepath.Base(epubPath))
	}
}

func TestCreateEPubWithMultipleChapters(t *testing.T) {
	_, _, cleanup := setupTestEPub(t)
	defer cleanup()

	// Create multiple chapter directories
	ch1Dir, _ := os.MkdirTemp("", "ch1-*")
	ch2Dir, _ := os.MkdirTemp("", "ch2-*")
	defer os.RemoveAll(ch1Dir)
	defer os.RemoveAll(ch2Dir)

	createTestImage(t, ch1Dir, "0001.png")
	createTestImage(t, ch2Dir, "0001.png")

	processor := NewEPubBuilder()

	manga := &data.Manga{
		ID:   "test-manga",
		Name: "Multi Chapter Test",
	}

	chapters := []*data.Chapter{
		{
			ID:         "ch-1",
			Title:      "Chapter 1",
			Volume:     "1",
			Number:     "1",
			Downloaded: true,
			FilePath:   ch1Dir,
		},
		{
			ID:         "ch-2",
			Title:      "Chapter 2",
			Volume:     "1",
			Number:     "2",
			Downloaded: true,
			FilePath:   ch2Dir,
		},
	}

	epubPath, err := processor.CreateEPub(manga, chapters)
	if err != nil {
		t.Fatalf("Failed to create EPub: %v", err)
	}

	if _, err := os.Stat(epubPath); os.IsNotExist(err) {
		t.Error("EPub file was not created")
	}
}

func TestCreateEPubNoChapters(t *testing.T) {
	_, _, cleanup := setupTestEPub(t)
	defer cleanup()

	processor := NewEPubBuilder()

	manga := &data.Manga{
		ID:   "test-manga",
		Name: "Empty Manga",
	}

	_, err := processor.CreateEPub(manga, []*data.Chapter{})
	if err == nil {
		t.Error("Expected error when creating EPub with no chapters")
	}
}

func TestCreateEPubSkipsNonDownloadedChapters(t *testing.T) {
	_, chapterDir, cleanup := setupTestEPub(t)
	defer cleanup()

	createTestImage(t, chapterDir, "0001.png")

	processor := NewEPubBuilder()

	manga := &data.Manga{
		ID:   "test-manga",
		Name: "Partial Download",
	}

	chapters := []*data.Chapter{
		{
			ID:         "ch-1",
			Number:     "1",
			Downloaded: true,
			FilePath:   chapterDir,
		},
		{
			ID:         "ch-2",
			Number:     "2",
			Downloaded: false, // Not downloaded
			FilePath:   "",
		},
	}

	// Should succeed with only the downloaded chapter
	epubPath, err := processor.CreateEPub(manga, chapters)
	if err != nil {
		t.Fatalf("Failed to create EPub: %v", err)
	}

	if _, err := os.Stat(epubPath); os.IsNotExist(err) {
		t.Error("EPub file was not created")
	}
}

func TestSanitizeFilename(t *testing.T) {
	tests := []struct {
		input    string
		expected string
	}{
		{"Normal Title", "Normal Title"},
		{"Title/With/Slashes", "Title_With_Slashes"},
		{"Title\\With\\Backslashes", "Title_With_Backslashes"},
		{"Title:With:Colons", "Title_With_Colons"},
		{"Title*With?Special<Chars>", "Title_With_Special_Chars_"},
		{"  Spaces Around  ", "Spaces Around"},
		{".Hidden File.", "Hidden File"},
	}

	for _, tt := range tests {
		result := sanitizeFilename(tt.input)
		if result != tt.expected {
			t.Errorf("sanitizeFilename(%q) = %q, expected %q", tt.input, result, tt.expected)
		}
	}
}

func TestIsImageFile(t *testing.T) {
	tests := []struct {
		filename string
		expected bool
	}{
		{"image.jpg", true},
		{"image.jpeg", true},
		{"image.png", true},
		{"image.gif", true},
		{"image.webp", true},
		{"image.JPG", true}, // Case insensitive
		{"document.pdf", false},
		{"text.txt", false},
		{"noextension", false},
		{"image.bmp", false},
	}

	for _, tt := range tests {
		result := isImageFile(tt.filename)
		if result != tt.expected {
			t.Errorf("isImageFile(%q) = %v, expected %v", tt.filename, result, tt.expected)
		}
	}
}

func TestCreateEPubWithInvalidDirectory(t *testing.T) {
	_, _, cleanup := setupTestEPub(t)
	defer cleanup()

	processor := NewEPubBuilder()

	manga := &data.Manga{
		ID:   "test-manga",
		Name: "Test",
	}

	chapters := []*data.Chapter{
		{
			ID:         "ch-1",
			Number:     "1",
			Downloaded: true,
			FilePath:   "/non/existent/path",
		},
	}

	_, err := processor.CreateEPub(manga, chapters)
	if err == nil {
		t.Error("Expected error when chapter directory doesn't exist")
	}
}
