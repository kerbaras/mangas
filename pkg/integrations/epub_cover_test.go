package integrations

import (
	"os"
	"strings"
	"testing"

	"github.com/kerbaras/mangas/pkg/data"
)

func TestEPubBuilder_SetMangaCover(t *testing.T) {
	builder := NewEPubBuilder(t.TempDir())
	manga := &data.Manga{ID: "manga-1", Name: "Test"}
	chapter := &data.Chapter{ID: "ch-1", Number: "1"}

	t.Run("set cover without init", func(t *testing.T) {
		cover := CoverData{
			Content:     []byte("cover-data"),
			ContentType: "image/jpeg",
		}
		err := builder.SetMangaCover(cover)
		if err == nil {
			t.Error("SetMangaCover() should fail when builder is not initialized")
		}
	})

	// Initialize for remaining tests
	if err := builder.Init(manga, chapter); err != nil {
		t.Fatalf("Init() failed: %v", err)
	}

	t.Run("set valid cover", func(t *testing.T) {
		cover := CoverData{
			Content:     []byte("cover-data"),
			ContentType: "image/jpeg",
		}
		err := builder.SetMangaCover(cover)
		if err != nil {
			t.Errorf("SetMangaCover() error = %v, want nil", err)
		}
		if builder.mangaCover == nil {
			t.Error("Manga cover should be set")
		}
	})

	t.Run("set empty cover", func(t *testing.T) {
		cover := CoverData{
			Content:     []byte{},
			ContentType: "image/jpeg",
		}
		err := builder.SetMangaCover(cover)
		if err == nil {
			t.Error("SetMangaCover() should fail with empty content")
		}
	})
}

func TestEPubBuilder_SetChapterCover(t *testing.T) {
	builder := NewEPubBuilder(t.TempDir())
	manga := &data.Manga{ID: "manga-1", Name: "Test"}
	chapter := &data.Chapter{ID: "ch-1", Number: "1"}

	t.Run("set cover without init", func(t *testing.T) {
		cover := CoverData{
			Content:     []byte("cover-data"),
			ContentType: "image/jpeg",
		}
		err := builder.SetChapterCover(cover)
		if err == nil {
			t.Error("SetChapterCover() should fail when builder is not initialized")
		}
	})

	// Initialize for remaining tests
	if err := builder.Init(manga, chapter); err != nil {
		t.Fatalf("Init() failed: %v", err)
	}

	t.Run("set valid cover", func(t *testing.T) {
		cover := CoverData{
			Content:     []byte("cover-data"),
			ContentType: "image/png",
		}
		err := builder.SetChapterCover(cover)
		if err != nil {
			t.Errorf("SetChapterCover() error = %v, want nil", err)
		}
		if builder.chapterCover == nil {
			t.Error("Chapter cover should be set")
		}
	})
}

func TestEPubBuilder_DoneWithCovers(t *testing.T) {
	outputDir := t.TempDir()
	builder := NewEPubBuilder(outputDir)
	
	manga := &data.Manga{
		ID:          "manga-1",
		Name:        "Test Manga",
		Description: "Test with covers",
	}
	chapter := &data.Chapter{
		ID:      "ch-1",
		MangaID: "manga-1",
		Number:  "1",
		Title:   "First Chapter",
	}

	if err := builder.Init(manga, chapter); err != nil {
		t.Fatalf("Init() failed: %v", err)
	}

	// Create test images
	pngData := createTestPNG()

	// Set manga cover
	mangaCover := CoverData{
		Content:     pngData,
		ContentType: "image/png",
	}
	if err := builder.SetMangaCover(mangaCover); err != nil {
		t.Fatalf("SetMangaCover() failed: %v", err)
	}

	// Set chapter cover
	chapterCover := CoverData{
		Content:     pngData,
		ContentType: "image/png",
	}
	if err := builder.SetChapterCover(chapterCover); err != nil {
		t.Fatalf("SetChapterCover() failed: %v", err)
	}

	// Add page images
	for i := 0; i < 3; i++ {
		img := ImageData{
			Content:     pngData,
			ContentType: "image/png",
			Index:       i,
		}
		if err := builder.Next(img); err != nil {
			t.Fatalf("Next() failed: %v", err)
		}
	}

	// Finalize EPUB
	path, err := builder.Done()
	if err != nil {
		t.Fatalf("Done() error = %v, want nil", err)
	}
	if path == "" {
		t.Error("Done() should return non-empty path")
	}

	// Verify EPUB file exists and is not empty
	info, err := os.Stat(path)
	if err != nil {
		t.Errorf("EPUB file should exist: %v", err)
	}
	if info.Size() == 0 {
		t.Error("EPUB file should not be empty")
	}
}

func TestEPubBuilder_TemplateRendering(t *testing.T) {
	builder := NewEPubBuilder(t.TempDir())
	
	if builder.templates == nil {
		t.Skip("Templates not loaded, skipping template test")
	}

	manga := &data.Manga{ID: "manga-1", Name: "Test"}
	chapter := &data.Chapter{
		ID:      "ch-1",
		Number:  "5",
		Volume:  "2",
		Title:   "Epic Battle",
		MangaID: "manga-1",
	}

	if err := builder.Init(manga, chapter); err != nil {
		t.Fatalf("Init() failed: %v", err)
	}

	// Create test pages
	pages := []PageData{
		{Path: "../images/page1.jpg", Index: 1, Alt: "Page 1"},
		{Path: "../images/page2.jpg", Index: 2, Alt: "Page 2"},
	}

	html, err := builder.renderChapterHTML("Vol. 2, Chapter 5: Epic Battle", pages)
	if err != nil {
		t.Fatalf("renderChapterHTML() failed: %v", err)
	}

	// Verify HTML contains expected elements
	if !strings.Contains(html, "Vol. 2, Chapter 5: Epic Battle") {
		t.Error("HTML should contain chapter title")
	}
	if !strings.Contains(html, "page1.jpg") {
		t.Error("HTML should contain page reference")
	}
	if !strings.Contains(html, "class=\"page\"") {
		t.Error("HTML should contain page divs")
	}
}

func TestEPubBuilder_SimpleFallback(t *testing.T) {
	builder := NewEPubBuilder(t.TempDir())
	builder.templates = nil // Force fallback to simple HTML

	pages := []PageData{
		{Path: "../images/test.jpg", Index: 1, Alt: "Test"},
	}

	html := builder.generateSimpleHTML("Test Chapter", pages)

	if !strings.Contains(html, "Test Chapter") {
		t.Error("Simple HTML should contain title")
	}
	if !strings.Contains(html, "test.jpg") {
		t.Error("Simple HTML should contain image path")
	}
}

func TestCoverData_ContentTypes(t *testing.T) {
	tests := []struct {
		contentType string
		want        string
	}{
		{"image/jpeg", ".jpg"},
		{"image/png", ".png"},
		{"image/gif", ".gif"},
		{"image/webp", ".webp"},
	}

	for _, tt := range tests {
		t.Run(tt.contentType, func(t *testing.T) {
			got := getExtensionFromContentType(tt.contentType)
			if got != tt.want {
				t.Errorf("getExtensionFromContentType(%q) = %q, want %q", tt.contentType, got, tt.want)
			}
		})
	}
}

func TestEPubBuilder_Integration_WithCovers(t *testing.T) {
	outputDir := t.TempDir()
	builder := NewEPubBuilder(outputDir)
	
	manga := &data.Manga{
		ID:          "int-test",
		Name:        "Integration Test",
		Description: "Full integration with covers",
	}
	chapter := &data.Chapter{
		ID:      "ch-int",
		MangaID: "int-test",
		Number:  "10",
		Volume:  "3",
		Title:   "Finale",
	}

	// Initialize
	if err := builder.Init(manga, chapter); err != nil {
		t.Fatalf("Init() failed: %v", err)
	}

	pngData := createTestPNG()

	// Add covers
	builder.SetMangaCover(CoverData{
		Content:     pngData,
		ContentType: "image/png",
	})
	builder.SetChapterCover(CoverData{
		Content:     pngData,
		ContentType: "image/png",
	})

	// Add multiple pages
	for i := 0; i < 10; i++ {
		builder.Next(ImageData{
			Content:     pngData,
			ContentType: "image/png",
			Index:       i,
		})
	}

	// Finalize
	path, err := builder.Done()
	if err != nil {
		t.Fatalf("Done() failed: %v", err)
	}

	// Verify output
	info, err := os.Stat(path)
	if err != nil {
		t.Fatalf("Failed to stat output file: %v", err)
	}

	// EPUB with 10 pages + 2 covers should be substantial
	if info.Size() < 1000 {
		t.Errorf("EPUB file seems too small: %d bytes", info.Size())
	}

	t.Logf("Created EPUB: %s (%d bytes)", path, info.Size())
}
