package integrations

import (
	"fmt"
	"os"
	"path/filepath"
	"sort"
	"strconv"
	"strings"

	"github.com/go-shiori/go-epub"
	"github.com/kerbaras/mangas/pkg/data"
)

type EPubBuilder struct {
	outputDir string
}

func NewEPubBuilder() *EPubBuilder {
	tempDir, _ := os.MkdirTemp("", "mangas-epub-*")
	return &EPubBuilder{outputDir: tempDir}
}

// CreateEPub compiles all chapters of a manga into a single EPub file
func (p *EPubBuilder) CreateEPub(manga *data.Manga, chapters []*data.Chapter) (string, error) {
	if len(chapters) == 0 {
		return "", fmt.Errorf("no chapters to compile")
	}

	// Ensure output directory exists
	if err := os.MkdirAll(p.outputDir, 0755); err != nil {
		return "", fmt.Errorf("failed to create output directory: %w", err)
	}

	// Sort chapters by volume and number
	sortedChapters := make([]*data.Chapter, len(chapters))
	copy(sortedChapters, chapters)
	sort.Slice(sortedChapters, func(i, j int) bool {
		vi, _ := strconv.ParseFloat(sortedChapters[i].Volume, 64)
		vj, _ := strconv.ParseFloat(sortedChapters[j].Volume, 64)
		if vi != vj {
			return vi < vj
		}
		ni, _ := strconv.ParseFloat(sortedChapters[i].Number, 64)
		nj, _ := strconv.ParseFloat(sortedChapters[j].Number, 64)
		return ni < nj
	})

	// Create EPub
	e, err := epub.NewEpub(manga.Name)
	if err != nil {
		return "", fmt.Errorf("failed to create EPub: %w", err)
	}

	// Set metadata
	e.SetAuthor("MangaDex")
	if manga.Description != "" {
		e.SetDescription(manga.Description)
	}
	e.SetLang("en")

	// Add chapters to EPub
	for _, chapter := range sortedChapters {
		if !chapter.Downloaded || chapter.FilePath == "" {
			continue
		}

		if err := p.addChapterToEPub(e, chapter); err != nil {
			return "", fmt.Errorf("failed to add chapter %s: %w", chapter.Number, err)
		}
	}

	// Generate output filename
	safeTitle := sanitizeFilename(manga.Name)
	outputPath := filepath.Join(p.outputDir, safeTitle+".epub")

	// Write EPub file
	if err := e.Write(outputPath); err != nil {
		return "", fmt.Errorf("failed to write EPub: %w", err)
	}

	return outputPath, nil
}

// addChapterToEPub adds a single chapter's images to the EPub
func (p *EPubBuilder) addChapterToEPub(e *epub.Epub, chapter *data.Chapter) error {
	// Read images from chapter directory
	files, err := os.ReadDir(chapter.FilePath)
	if err != nil {
		return fmt.Errorf("failed to read chapter directory: %w", err)
	}

	// Filter and sort image files
	var imageFiles []os.DirEntry
	for _, file := range files {
		if !file.IsDir() && isImageFile(file.Name()) {
			imageFiles = append(imageFiles, file)
		}
	}

	if len(imageFiles) == 0 {
		return fmt.Errorf("no images found in chapter directory")
	}

	sort.Slice(imageFiles, func(i, j int) bool {
		return imageFiles[i].Name() < imageFiles[j].Name()
	})

	// Create chapter title
	chapterTitle := fmt.Sprintf("Chapter %s", chapter.Number)
	if chapter.Volume != "" && chapter.Volume != "0" {
		chapterTitle = fmt.Sprintf("Vol. %s, %s", chapter.Volume, chapterTitle)
	}
	if chapter.Title != "" {
		chapterTitle = fmt.Sprintf("%s: %s", chapterTitle, chapter.Title)
	}

	// Build HTML content for chapter
	var htmlContent strings.Builder
	htmlContent.WriteString(fmt.Sprintf("<h1>%s</h1>\n", chapterTitle))

	for i, imgFile := range imageFiles {
		imgPath := filepath.Join(chapter.FilePath, imgFile.Name())

		// Add image to EPub
		internalPath, err := e.AddImage(imgPath, "")
		if err != nil {
			return fmt.Errorf("failed to add image %s: %w", imgFile.Name(), err)
		}

		// Add image to HTML content
		htmlContent.WriteString(fmt.Sprintf(
			`<div class="page"><img src="%s" alt="Page %d" style="width:100%%;height:auto;"/></div>%s`,
			internalPath, i+1, "\n",
		))
	}

	// Add chapter section to EPub
	_, err = e.AddSection(htmlContent.String(), chapterTitle, "", "")
	if err != nil {
		return fmt.Errorf("failed to add section: %w", err)
	}

	return nil
}

// isImageFile checks if a file has an image extension
func isImageFile(filename string) bool {
	ext := strings.ToLower(filepath.Ext(filename))
	return ext == ".jpg" || ext == ".jpeg" || ext == ".png" || ext == ".gif" || ext == ".webp"
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
