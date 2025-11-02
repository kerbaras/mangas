package integrations

import (
	"bytes"
	"fmt"
	"html/template"
	"os"
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

// CoverData represents cover image data
type CoverData struct {
	Content     []byte
	ContentType string
}

// EPubBuilder builds EPUB files by streaming images
type EPubBuilder struct {
	outputDir   string
	tempDir     string
	epub        *epub.Epub
	manga       *data.Manga
	chapter     *data.Chapter
	images      []ImageData
	chapterCover *CoverData
	mangaCover   *CoverData
	templates   *template.Template
}

// Template data structures
type ChapterTemplateData struct {
	Title       string
	Volume      string
	Number      string
	ChapterTitle string
	Pages       []PageData
	HasCover    bool
}

type PageData struct {
	Path  string
	Index int
	Alt   string
}

// HTML templates for EPUB content
const chapterTemplate = `<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops">
<head>
    <title>{{.Title}}</title>
    <style>
        body {
            margin: 0;
            padding: 0;
            text-align: center;
        }
        .chapter-title {
            font-size: 2em;
            font-weight: bold;
            margin: 1em 0;
            page-break-after: always;
        }
        .cover-page {
            page-break-after: always;
        }
        .page {
            page-break-after: always;
            margin: 0;
            padding: 0;
        }
        img {
            max-width: 100%;
            max-height: 100vh;
            height: auto;
            width: auto;
            display: block;
            margin: 0 auto;
        }
    </style>
</head>
<body>
    <div class="chapter-title">
        <h1>{{.Title}}</h1>
    </div>
    {{if .HasCover}}
    <div class="cover-page">
        <img src="{{index .Pages 0 | .Path}}" alt="Chapter Cover"/>
    </div>
    {{end}}
    {{range .Pages}}
    <div class="page">
        <img src="{{.Path}}" alt="{{.Alt}}" />
    </div>
    {{end}}
</body>
</html>`

// NewEPubBuilder creates a new EPubBuilder
func NewEPubBuilder(outputDir string) *EPubBuilder {
	// Parse templates
	tmpl, err := template.New("chapter").Parse(chapterTemplate)
	if err != nil {
		// Fallback to nil, will use simple HTML generation
		tmpl = nil
	}

	return &EPubBuilder{
		outputDir: outputDir,
		images:    make([]ImageData, 0),
		templates: tmpl,
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

	// Create temporary directory for staging images
	tempDir, err := os.MkdirTemp("", "manga-epub-*")
	if err != nil {
		return fmt.Errorf("failed to create temp directory: %w", err)
	}

	b.manga = manga
	b.chapter = chapter
	b.tempDir = tempDir
	b.images = make([]ImageData, 0)
	b.chapterCover = nil
	b.mangaCover = nil

	// Create EPub
	e, err := epub.NewEpub(manga.Name)
	if err != nil {
		os.RemoveAll(tempDir)
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

// SetMangaCover sets the manga cover image
func (b *EPubBuilder) SetMangaCover(cover CoverData) error {
	if b.epub == nil {
		return fmt.Errorf("builder not initialized, call Init first")
	}
	if len(cover.Content) == 0 {
		return fmt.Errorf("cover content is empty")
	}
	b.mangaCover = &cover
	return nil
}

// SetChapterCover sets the chapter cover image
func (b *EPubBuilder) SetChapterCover(cover CoverData) error {
	if b.epub == nil {
		return fmt.Errorf("builder not initialized, call Init first")
	}
	if len(cover.Content) == 0 {
		return fmt.Errorf("cover content is empty")
	}
	b.chapterCover = &cover
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

	defer func() {
		// Clean up temp directory
		if b.tempDir != "" {
			os.RemoveAll(b.tempDir)
		}
	}()

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

	// Add manga cover if provided
	if b.mangaCover != nil {
		coverPath, err := b.addCoverImage(b.mangaCover, "manga_cover")
		if err == nil {
			// Set as EPUB cover
			b.epub.SetCover(coverPath, "")
		}
	}

	// Prepare template data
	var pages []PageData

	// Add chapter cover if provided
	if b.chapterCover != nil {
		coverPath, err := b.addCoverImage(b.chapterCover, "chapter_cover")
		if err == nil {
			pages = append(pages, PageData{
				Path:  coverPath,
				Index: -1,
				Alt:   "Chapter Cover",
			})
		}
	}

	// Write images to temp directory and add to EPUB
	for i, img := range b.images {
		ext := getExtensionFromContentType(img.ContentType)
		filename := fmt.Sprintf("page_%04d%s", img.Index, ext)
		
		// Write image to temp file
		tempFilePath := filepath.Join(b.tempDir, filename)
		if err := os.WriteFile(tempFilePath, img.Content, 0644); err != nil {
			return "", fmt.Errorf("failed to write temp image %d: %w", img.Index, err)
		}

		// Add image from temp file
		internalPath, err := b.epub.AddImage(tempFilePath, filename)
		if err != nil {
			return "", fmt.Errorf("failed to add image %d to EPUB: %w", img.Index, err)
		}

		pages = append(pages, PageData{
			Path:  internalPath,
			Index: i + 1,
			Alt:   fmt.Sprintf("Page %d", i+1),
		})
	}

	// Generate HTML content using templates
	var htmlContent string
	var htmlErr error
	if b.templates != nil {
		htmlContent, htmlErr = b.renderChapterHTML(chapterTitle, pages)
		if htmlErr != nil {
			// Fallback to simple HTML generation
			htmlContent = b.generateSimpleHTML(chapterTitle, pages)
		}
	} else {
		htmlContent = b.generateSimpleHTML(chapterTitle, pages)
	}

	// Add chapter section to EPub
	_, err := b.epub.AddSection(htmlContent, chapterTitle, "", "")
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
	b.chapterCover = nil
	b.mangaCover = nil
	b.tempDir = ""

	return outputPath, nil
}

// addCoverImage adds a cover image to the EPUB and returns its internal path
func (b *EPubBuilder) addCoverImage(cover *CoverData, prefix string) (string, error) {
	ext := getExtensionFromContentType(cover.ContentType)
	filename := fmt.Sprintf("%s%s", prefix, ext)
	
	tempFilePath := filepath.Join(b.tempDir, filename)
	if err := os.WriteFile(tempFilePath, cover.Content, 0644); err != nil {
		return "", fmt.Errorf("failed to write cover image: %w", err)
	}

	internalPath, err := b.epub.AddImage(tempFilePath, filename)
	if err != nil {
		return "", fmt.Errorf("failed to add cover to EPUB: %w", err)
	}

	return internalPath, nil
}

// renderChapterHTML renders the chapter HTML using templates
func (b *EPubBuilder) renderChapterHTML(title string, pages []PageData) (string, error) {
	data := ChapterTemplateData{
		Title:        title,
		Volume:       b.chapter.Volume,
		Number:       b.chapter.Number,
		ChapterTitle: b.chapter.Title,
		Pages:        pages,
		HasCover:     b.chapterCover != nil,
	}

	var buf bytes.Buffer
	if err := b.templates.Execute(&buf, data); err != nil {
		return "", err
	}

	return buf.String(), nil
}

// generateSimpleHTML generates simple HTML when templates fail
func (b *EPubBuilder) generateSimpleHTML(title string, pages []PageData) string {
	var html strings.Builder
	html.WriteString(fmt.Sprintf("<h1>%s</h1>\n", title))

	for _, page := range pages {
		html.WriteString(fmt.Sprintf(
			`<div class="page"><img src="%s" alt="%s" style="width:100%%;height:auto;"/></div>%s`,
			page.Path, page.Alt, "\n",
		))
	}

	return html.String()
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
