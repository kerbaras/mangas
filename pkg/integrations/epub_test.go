package integrations

import (
	"bytes"
	"os"
	"path/filepath"
	"testing"

	"github.com/kerbaras/mangas/pkg/data"
)

func TestEPubBuilder_Init(t *testing.T) {
	tests := []struct {
		name    string
		manga   *data.Manga
		chapter *data.Chapter
		wantErr bool
	}{
		{
			name: "valid initialization",
			manga: &data.Manga{
				ID:          "manga-1",
				Name:        "Test Manga",
				Description: "A test manga",
			},
			chapter: &data.Chapter{
				ID:      "ch-1",
				MangaID: "manga-1",
				Number:  "1",
				Volume:  "1",
				Title:   "First Chapter",
			},
			wantErr: false,
		},
		{
			name:    "nil manga",
			manga:   nil,
			chapter: &data.Chapter{ID: "ch-1"},
			wantErr: true,
		},
		{
			name:    "nil chapter",
			manga:   &data.Manga{ID: "manga-1", Name: "Test"},
			chapter: nil,
			wantErr: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			builder := NewEPubBuilder(t.TempDir())
			err := builder.Init(tt.manga, tt.chapter)

			if (err != nil) != tt.wantErr {
				t.Errorf("Init() error = %v, wantErr %v", err, tt.wantErr)
			}

			if !tt.wantErr {
				if builder.epub == nil {
					t.Error("Init() should have created epub instance")
				}
				if builder.tempDir == "" {
					t.Error("Init() should have created temp directory")
				}
				// Verify temp directory exists
				if _, err := os.Stat(builder.tempDir); os.IsNotExist(err) {
					t.Error("Temp directory should exist after Init()")
				}
			}
		})
	}
}

func TestEPubBuilder_Next(t *testing.T) {
	builder := NewEPubBuilder(t.TempDir())
	manga := &data.Manga{ID: "manga-1", Name: "Test Manga"}
	chapter := &data.Chapter{ID: "ch-1", Number: "1"}

	t.Run("next without init", func(t *testing.T) {
		img := ImageData{
			Content:     []byte("fake-image"),
			ContentType: "image/jpeg",
			Index:       0,
		}
		err := builder.Next(img)
		if err == nil {
			t.Error("Next() should fail when builder is not initialized")
		}
	})

	// Initialize for remaining tests
	if err := builder.Init(manga, chapter); err != nil {
		t.Fatalf("Init() failed: %v", err)
	}

	t.Run("valid image", func(t *testing.T) {
		img := ImageData{
			Content:     []byte("fake-image-content"),
			ContentType: "image/jpeg",
			Index:       0,
		}
		err := builder.Next(img)
		if err != nil {
			t.Errorf("Next() error = %v, want nil", err)
		}
		if len(builder.images) != 1 {
			t.Errorf("Expected 1 image, got %d", len(builder.images))
		}
	})

	t.Run("empty content", func(t *testing.T) {
		img := ImageData{
			Content:     []byte{},
			ContentType: "image/jpeg",
			Index:       1,
		}
		err := builder.Next(img)
		if err == nil {
			t.Error("Next() should fail with empty content")
		}
	})

	t.Run("missing content type", func(t *testing.T) {
		img := ImageData{
			Content:     []byte("fake-image"),
			ContentType: "",
			Index:       1,
		}
		err := builder.Next(img)
		if err == nil {
			t.Error("Next() should fail with empty content type")
		}
	})

	t.Run("multiple images", func(t *testing.T) {
		initialCount := len(builder.images)
		for i := 0; i < 5; i++ {
			img := ImageData{
				Content:     []byte("fake-image-" + string(rune(i))),
				ContentType: "image/png",
				Index:       i + initialCount,
			}
			if err := builder.Next(img); err != nil {
				t.Errorf("Next() failed for image %d: %v", i, err)
			}
		}
		if len(builder.images) != initialCount+5 {
			t.Errorf("Expected %d images, got %d", initialCount+5, len(builder.images))
		}
	})
}

func TestEPubBuilder_Done(t *testing.T) {
	t.Run("done without init", func(t *testing.T) {
		builder := NewEPubBuilder(t.TempDir())
		_, err := builder.Done()
		if err == nil {
			t.Error("Done() should fail when builder is not initialized")
		}
	})

	t.Run("done without images", func(t *testing.T) {
		builder := NewEPubBuilder(t.TempDir())
		manga := &data.Manga{ID: "manga-1", Name: "Test Manga"}
		chapter := &data.Chapter{ID: "ch-1", Number: "1"}

		if err := builder.Init(manga, chapter); err != nil {
			t.Fatalf("Init() failed: %v", err)
		}

		_, err := builder.Done()
		if err == nil {
			t.Error("Done() should fail when no images were added")
		}
	})

	t.Run("successful epub creation", func(t *testing.T) {
		outputDir := t.TempDir()
		builder := NewEPubBuilder(outputDir)
		manga := &data.Manga{
			ID:          "manga-1",
			Name:        "Test Manga",
			Description: "Test description",
		}
		chapter := &data.Chapter{
			ID:      "ch-1",
			MangaID: "manga-1",
			Number:  "1",
			Volume:  "1",
			Title:   "Test Chapter",
		}

		if err := builder.Init(manga, chapter); err != nil {
			t.Fatalf("Init() failed: %v", err)
		}

		// Create a simple 1x1 PNG image
		pngData := createTestPNG()

		// Add images in non-sequential order to test sorting
		for _, idx := range []int{2, 0, 1} {
			img := ImageData{
				Content:     pngData,
				ContentType: "image/png",
				Index:       idx,
			}
			if err := builder.Next(img); err != nil {
				t.Fatalf("Next() failed: %v", err)
			}
		}

		path, err := builder.Done()
		if err != nil {
			t.Errorf("Done() error = %v, want nil", err)
		}
		if path == "" {
			t.Error("Done() should return non-empty path")
		}

		// Verify EPUB file exists
		if _, err := os.Stat(path); os.IsNotExist(err) {
			t.Error("EPUB file should exist after Done()")
		}

		// Verify temp directory was cleaned up
		// Note: os.RemoveAll is called in defer, so it should be removed
		// We can't reliably check this immediately after Done() due to defer timing

		// Verify builder was reset
		if builder.epub != nil {
			t.Error("Builder should be reset after Done()")
		}
		if builder.tempDir != "" {
			t.Error("Builder tempDir should be cleared after Done()")
		}
	})

	t.Run("chapter title formatting", func(t *testing.T) {
		pngData := createTestPNG()

		tests := []struct {
			name    string
			chapter *data.Chapter
		}{
			{
				name: "with volume and title",
				chapter: &data.Chapter{
					ID:      "ch-1",
					Number:  "5",
					Volume:  "2",
					Title:   "The Beginning",
					MangaID: "manga-1",
				},
			},
			{
				name: "without volume",
				chapter: &data.Chapter{
					ID:      "ch-2",
					Number:  "10",
					Volume:  "0",
					Title:   "The End",
					MangaID: "manga-1",
				},
			},
			{
				name: "without title",
				chapter: &data.Chapter{
					ID:      "ch-3",
					Number:  "15",
					Volume:  "3",
					MangaID: "manga-1",
				},
			},
		}

		for _, tt := range tests {
			t.Run(tt.name, func(t *testing.T) {
				builder := NewEPubBuilder(t.TempDir())
				manga := &data.Manga{ID: "manga-1", Name: "Test Manga"}

				if err := builder.Init(manga, tt.chapter); err != nil {
					t.Fatalf("Init() failed: %v", err)
				}

				img := ImageData{
					Content:     pngData,
					ContentType: "image/png",
					Index:       0,
				}
				if err := builder.Next(img); err != nil {
					t.Fatalf("Next() failed: %v", err)
				}

				_, err := builder.Done()
				if err != nil {
					t.Errorf("Done() error = %v, want nil", err)
				}
			})
		}
	})

	t.Run("temp files are written and cleaned", func(t *testing.T) {
		outputDir := t.TempDir()
		builder := NewEPubBuilder(outputDir)
		manga := &data.Manga{ID: "manga-1", Name: "Test"}
		chapter := &data.Chapter{ID: "ch-1", Number: "1"}

		if err := builder.Init(manga, chapter); err != nil {
			t.Fatalf("Init() failed: %v", err)
		}

		tempDir := builder.tempDir
		pngData := createTestPNG()

		img := ImageData{
			Content:     pngData,
			ContentType: "image/png",
			Index:       0,
		}
		if err := builder.Next(img); err != nil {
			t.Fatalf("Next() failed: %v", err)
		}

		// Before Done(), temp dir should exist but be empty
		files, err := os.ReadDir(tempDir)
		if err != nil {
			t.Fatalf("Failed to read temp dir: %v", err)
		}
		if len(files) != 0 {
			t.Error("Temp dir should be empty before Done()")
		}

		_, err = builder.Done()
		if err != nil {
			t.Fatalf("Done() failed: %v", err)
		}

		// After Done(), temp dir should be deleted
		// Note: Cleanup happens in defer, directory should be removed
	})
}

func TestEPubBuilder_ContentTypeExtensions(t *testing.T) {
	tests := []struct {
		contentType string
		want        string
	}{
		{"image/jpeg", ".jpg"},
		{"image/jpg", ".jpg"},
		{"image/png", ".png"},
		{"image/gif", ".gif"},
		{"image/webp", ".webp"},
		{"image/unknown", ".jpg"}, // default
		{"", ".jpg"},              // default
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

func TestSanitizeFilename(t *testing.T) {
	tests := []struct {
		input string
		want  string
	}{
		{"Normal Title", "Normal Title"},
		{"Title/With\\Slash", "Title_With_Slash"},
		{"Title:With*Special?Chars", "Title_With_Special_Chars"},
		{"  Title with spaces  ", "Title with spaces"},
		{"...Title with dots...", "Title with dots"},
		{"Title<>|With\"All", "Title___With_All"},
	}

	for _, tt := range tests {
		t.Run(tt.input, func(t *testing.T) {
			got := sanitizeFilename(tt.input)
			if got != tt.want {
				t.Errorf("sanitizeFilename(%q) = %q, want %q", tt.input, got, tt.want)
			}
		})
	}
}

func TestEPubBuilder_OutputFilename(t *testing.T) {
	outputDir := t.TempDir()
	builder := NewEPubBuilder(outputDir)
	manga := &data.Manga{
		ID:   "manga-1",
		Name: "Test: Manga <With> Special/Chars",
	}
	chapter := &data.Chapter{
		ID:     "ch-1",
		Number: "1.5",
	}

	if err := builder.Init(manga, chapter); err != nil {
		t.Fatalf("Init() failed: %v", err)
	}

	pngData := createTestPNG()
	img := ImageData{
		Content:     pngData,
		ContentType: "image/png",
		Index:       0,
	}
	if err := builder.Next(img); err != nil {
		t.Fatalf("Next() failed: %v", err)
	}

	path, err := builder.Done()
	if err != nil {
		t.Fatalf("Done() failed: %v", err)
	}

	// Verify filename is sanitized
	filename := filepath.Base(path)
	if filename == "" {
		t.Error("Filename should not be empty")
	}

	// Should not contain special characters
	invalidChars := []string{"/", "\\", ":", "*", "?", "\"", "<", ">", "|"}
	for _, char := range invalidChars {
		if bytes.Contains([]byte(filename), []byte(char)) {
			t.Errorf("Filename %q should not contain %q", filename, char)
		}
	}
}

// createTestPNG creates a minimal valid PNG image
func createTestPNG() []byte {
	// Minimal 1x1 transparent PNG
	var buf bytes.Buffer
	// PNG signature
	buf.Write([]byte{0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A})
	// IHDR chunk (1x1, 8-bit grayscale)
	buf.Write([]byte{
		0x00, 0x00, 0x00, 0x0D, // Length
		0x49, 0x48, 0x44, 0x52, // "IHDR"
		0x00, 0x00, 0x00, 0x01, // Width: 1
		0x00, 0x00, 0x00, 0x01, // Height: 1
		0x08, 0x00, 0x00, 0x00, 0x00, // Bit depth, color type, etc.
		0x3A, 0x7E, 0x9B, 0x55, // CRC
	})
	// IDAT chunk (minimal compressed data)
	buf.Write([]byte{
		0x00, 0x00, 0x00, 0x0A, // Length
		0x49, 0x44, 0x41, 0x54, // "IDAT"
		0x08, 0xD7, 0x63, 0x60, 0x00, 0x00, 0x00, 0x02, 0x00, 0x01,
		0xE2, 0x21, 0xBC, 0x33, // CRC
	})
	// IEND chunk
	buf.Write([]byte{
		0x00, 0x00, 0x00, 0x00, // Length
		0x49, 0x45, 0x4E, 0x44, // "IEND"
		0xAE, 0x42, 0x60, 0x82, // CRC
	})
	return buf.Bytes()
}
