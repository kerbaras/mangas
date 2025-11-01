package integrations

import (
	"encoding/base64"
	"fmt"
	"path/filepath"
	"sort"
	"strings"

	"github.com/go-shiori/go-epub"
	"github.com/kerbaras/mangas/pkg/data"
)

// ImageData represents an image with its content and metadata
type ImageData struct {
	Content     []byte
	ContentType string // e.g., "image/jpeg", "image/png"
	Index       int    // Page number/order
}

// EPubBuilder builds EPUB files by streaming images
type EPubBuilder struct {
	outputDir string
	epub      *epub.Epub
	manga     *data.Manga
	chapter   *data.Chapter
	images    []ImageData
}

// NewEPubBuilder creates a new EPubBuilder
func NewEPubBuilder(outputDir string) *EPubBuilder {
	return &EPubBuilder{
		outputDir: outputDir,
		images:    make([]ImageData, 0),
	}
}

// Init initializes the builder for a specific chapter
func (b *EPubBuilder) Init(manga *data.Manga, chapter *data.Chapter) error {
	if manga == nil {
		return fmt.Errorf("manga cannot be nil")
	}
	if chapter == nil {
		return fmt.Errorf("chapter cannot be nil")
	}

	b.manga = manga
	b.chapter = chapter
	b.images = make([]ImageData, 0)

	// Create EPub
	e, err := epub.NewEpub(manga.Name)
	if err != nil {
		return fmt.Errorf("failed to create EPub: %w", err)
	}

	// Set metadata
	e.SetAuthor("MangaDex")
	if manga.Description != "" {
		e.SetDescription(manga.Description)
	}
	e.SetLang("en")

	b.epub = e
	return nil
}

// Next adds an image to the chapter
func (b *EPubBuilder) Next(image ImageData) error {
	if b.epub == nil {
		return fmt.Errorf("builder not initialized, call Init first")
	}
	if len(image.Content) == 0 {
		return fmt.Errorf("image content is empty")
	}
	if image.ContentType == "" {
		return fmt.Errorf("image content type is required")
	}

	b.images = append(b.images, image)
	return nil
}

// Done finalizes and writes the EPUB file
func (b *EPubBuilder) Done() (string, error) {
	if b.epub == nil {
		return "", fmt.Errorf("builder not initialized, call Init first")
	}
	if len(b.images) == 0 {
		return "", fmt.Errorf("no images added to chapter")
	}

	// Sort images by index
	sort.Slice(b.images, func(i, j int) bool {
		return b.images[i].Index < b.images[j].Index
	})

	// Create chapter title
	chapterTitle := fmt.Sprintf("Chapter %s", b.chapter.Number)
	if b.chapter.Volume != "" && b.chapter.Volume != "0" {
		chapterTitle = fmt.Sprintf("Vol. %s, %s", b.chapter.Volume, chapterTitle)
	}
	if b.chapter.Title != "" {
		chapterTitle = fmt.Sprintf("%s: %s", chapterTitle, b.chapter.Title)
	}

	// Build HTML content for chapter
	var htmlContent strings.Builder
	htmlContent.WriteString(fmt.Sprintf("<h1>%s</h1>\n", chapterTitle))

	// Add all images to EPUB
	for i, img := range b.images {
		// Determine file extension from content type
		ext := getExtensionFromContentType(img.ContentType)
		filename := fmt.Sprintf("page_%04d%s", img.Index, ext)

		// Create data URL from image content
		b64 := base64.StdEncoding.EncodeToString(img.Content)
		dataURL := fmt.Sprintf("data:%s;base64,%s", img.ContentType, b64)

		// Add image from data URL
		internalPath, err := b.epub.AddImage(dataURL, filename)
		if err != nil {
			return "", fmt.Errorf("failed to add image %d: %w", img.Index, err)
		}

		// Add image to HTML content
		htmlContent.WriteString(fmt.Sprintf(
			`<div class="page"><img src="%s" alt="Page %d" style="width:100%%;height:auto;"/></div>%s`,
			internalPath, i+1, "\n",
		))
	}

	// Add chapter section to EPub
	_, err := b.epub.AddSection(htmlContent.String(), chapterTitle, "", "")
	if err != nil {
		return "", fmt.Errorf("failed to add section: %w", err)
	}

	// Generate output filename
	safeTitle := sanitizeFilename(b.manga.Name)
	safeCh := sanitizeFilename(fmt.Sprintf("ch_%s", b.chapter.Number))
	outputPath := filepath.Join(b.outputDir, fmt.Sprintf("%s_%s.epub", safeTitle, safeCh))

	// Write EPub file
	if err := b.epub.Write(outputPath); err != nil {
		return "", fmt.Errorf("failed to write EPub: %w", err)
	}

	// Reset for next use
	b.epub = nil
	b.manga = nil
	b.chapter = nil
	b.images = nil

	return outputPath, nil
}

// getExtensionFromContentType returns the file extension for a given content type
func getExtensionFromContentType(contentType string) string {
	switch contentType {
	case "image/jpeg", "image/jpg":
		return ".jpg"
	case "image/png":
		return ".png"
	case "image/gif":
		return ".gif"
	case "image/webp":
		return ".webp"
	default:
		return ".jpg"
	}
}

// sanitizeFilename removes characters that are invalid in filenames
func sanitizeFilename(name string) string {
	// Replace invalid characters with underscores
	invalid := []string{"/", "\\", ":", "*", "?", "\"", "<", ">", "|"}
	result := name
	for _, char := range invalid {
		result = strings.ReplaceAll(result, char, "_")
	}
	// Trim spaces and dots from ends
	result = strings.TrimSpace(result)
	result = strings.Trim(result, ".")
	return result
}
