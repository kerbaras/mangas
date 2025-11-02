package integrations

import (
	"archive/zip"
	"bytes"
	"fmt"
	"html/template"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"sort"
	"strings"

	"github.com/kerbaras/mangas/pkg/data"
)

// KindleConverter converts manga EPUBs to Kindle-optimized format
type KindleConverter struct {
	device    KindleDevice
	processor *ImageProcessor
	tempDir   string
}

// NewKindleConverter creates a new Kindle converter for the specified device
func NewKindleConverter(deviceID string) (*KindleConverter, error) {
	device, ok := GetDeviceProfile(deviceID)
	if !ok {
		return nil, fmt.Errorf("unknown device: %s", deviceID)
	}

	settings := device.GetOptimizationSettings()
	processor := NewImageProcessor(settings)

	tempDir, err := os.MkdirTemp("", "kindle-convert-*")
	if err != nil {
		return nil, fmt.Errorf("failed to create temp directory: %w", err)
	}

	return &KindleConverter{
		device:    device,
		processor: processor,
		tempDir:   tempDir,
	}, nil
}

// ConvertChapters converts multiple chapter EPUBs into a single Kindle-optimized file
func (c *KindleConverter) ConvertChapters(options ExportOptions) (string, error) {
	if len(options.Chapters) == 0 {
		return "", fmt.Errorf("no chapters provided")
	}

	// Create output directory if needed
	outputDir := filepath.Dir(options.OutputPath)
	if err := os.MkdirAll(outputDir, 0755); err != nil {
		return "", fmt.Errorf("failed to create output directory: %w", err)
	}

	// Extract and process all chapter images
	allImages := make([]ProcessedImage, 0)
	chapterTitles := make([]string, 0)

	for i, chapterPath := range options.Chapters {
		images, title, err := c.extractAndProcessChapter(chapterPath, i)
		if err != nil {
			return "", fmt.Errorf("failed to process chapter %s: %w", chapterPath, err)
		}
		allImages = append(allImages, images...)
		chapterTitles = append(chapterTitles, title)
	}

	// Generate Kindle-optimized EPUB
	epubPath, err := c.generateOptimizedEPUB(allImages, chapterTitles, options)
	if err != nil {
		return "", fmt.Errorf("failed to generate EPUB: %w", err)
	}

	// Convert to requested format if not EPUB
	if options.Format != "epub" && options.Format != "" {
		convertedPath, err := c.convertFormat(epubPath, options)
		if err != nil {
			return "", fmt.Errorf("failed to convert format: %w", err)
		}
		return convertedPath, nil
	}

	return epubPath, nil
}

// ProcessedImage represents a processed manga page
type ProcessedImage struct {
	Data         []byte
	ChapterIndex int
	PageIndex    int
	Filename     string
}

// extractAndProcessChapter extracts images from an EPUB and processes them
func (c *KindleConverter) extractAndProcessChapter(epubPath string, chapterIndex int) ([]ProcessedImage, string, error) {
	// Open EPUB as ZIP
	reader, err := zip.OpenReader(epubPath)
	if err != nil {
		return nil, "", fmt.Errorf("failed to open EPUB: %w", err)
	}
	defer reader.Close()

	images := make([]ProcessedImage, 0)
	chapterTitle := fmt.Sprintf("Chapter %d", chapterIndex+1)

	// Find all images in the EPUB
	for _, file := range reader.File {
		// Check if file is an image
		if !strings.HasSuffix(strings.ToLower(file.Name), ".jpg") &&
			!strings.HasSuffix(strings.ToLower(file.Name), ".jpeg") &&
			!strings.HasSuffix(strings.ToLower(file.Name), ".png") {
			continue
		}

		// Skip cover images (we'll handle them separately)
		if strings.Contains(strings.ToLower(file.Name), "cover") {
			continue
		}

		// Extract image
		rc, err := file.Open()
		if err != nil {
			continue
		}

		imageData, err := io.ReadAll(rc)
		rc.Close()
		if err != nil {
			continue
		}

		// Process image for Kindle
		processed, err := c.processor.ProcessImageData(imageData)
		if err != nil {
			// Log error but continue with other images
			continue
		}

		images = append(images, ProcessedImage{
			Data:         processed,
			ChapterIndex: chapterIndex,
			PageIndex:    len(images),
			Filename:     filepath.Base(file.Name),
		})
	}

	// Sort images by filename to maintain order
	sort.Slice(images, func(i, j int) bool {
		return images[i].Filename < images[j].Filename
	})

	// Update page indices after sorting
	for i := range images {
		images[i].PageIndex = i
	}

	return images, chapterTitle, nil
}

// generateOptimizedEPUB creates a Kindle-optimized EPUB
func (c *KindleConverter) generateOptimizedEPUB(images []ProcessedImage, chapterTitles []string, options ExportOptions) (string, error) {
	epubBuilder := NewEPubBuilder(filepath.Dir(options.OutputPath))

	// Create a synthetic manga entry
	manga := &data.Manga{
		ID:          "kindle-export",
		Name:        options.Title,
		Description: fmt.Sprintf("Optimized for %s", c.device.Name),
	}

	// Create a synthetic chapter
	chapter := &data.Chapter{
		ID:      "combined",
		MangaID: "kindle-export",
		Number:  "1",
		Title:   "Complete Volume",
	}

	if err := epubBuilder.Init(manga, chapter); err != nil {
		return "", err
	}

	// Add all processed images
	for _, img := range images {
		imageData := ImageData{
			Content:     img.Data,
			ContentType: "image/jpeg",
			Index:       img.ChapterIndex*1000 + img.PageIndex,
		}
		if err := epubBuilder.Next(imageData); err != nil {
			return "", err
		}
	}

	// Generate EPUB
	epubPath, err := epubBuilder.Done()
	if err != nil {
		return "", err
	}

	return epubPath, nil
}

// convertFormat converts EPUB to MOBI or other Kindle formats
func (c *KindleConverter) convertFormat(epubPath string, options ExportOptions) (string, error) {
	// Determine output filename
	ext := string(options.Format)
	outputPath := strings.TrimSuffix(options.OutputPath, filepath.Ext(options.OutputPath)) + "." + ext

	// Try using ebook-convert from Calibre (most common)
	if err := c.convertWithCalibre(epubPath, outputPath, options); err == nil {
		return outputPath, nil
	}

	// Try using kindlegen (Amazon's tool, deprecated but still works)
	if options.Format == FormatMOBI {
		if err := c.convertWithKindlegen(epubPath, outputPath); err == nil {
			return outputPath, nil
		}
	}

	// If all conversion methods fail, return the EPUB
	return epubPath, fmt.Errorf("no conversion tool available (tried ebook-convert, kindlegen). Please install Calibre or use EPUB format")
}

// convertWithCalibre uses Calibre's ebook-convert tool
func (c *KindleConverter) convertWithCalibre(input, output string, options ExportOptions) error {
	args := []string{
		input,
		output,
		"--output-profile", "kindle",
		"--no-inline-toc",
	}

	// Add metadata
	if options.Title != "" {
		args = append(args, "--title", options.Title)
	}
	if options.Author != "" {
		args = append(args, "--authors", options.Author)
	}

	// Right-to-left for manga
	if options.RightToLeft {
		args = append(args, "--page-progression-direction", "rtl")
	}

	cmd := exec.Command("ebook-convert", args...)
	if err := cmd.Run(); err != nil {
		return fmt.Errorf("ebook-convert failed: %w", err)
	}

	return nil
}

// convertWithKindlegen uses Amazon's kindlegen tool
func (c *KindleConverter) convertWithKindlegen(input, output string) error {
	cmd := exec.Command("kindlegen", input, "-o", filepath.Base(output))
	cmd.Dir = filepath.Dir(input)
	
	if err := cmd.Run(); err != nil {
		return fmt.Errorf("kindlegen failed: %w", err)
	}

	// kindlegen creates output in the same directory as input
	generatedPath := strings.TrimSuffix(input, filepath.Ext(input)) + ".mobi"
	if generatedPath != output {
		if err := os.Rename(generatedPath, output); err != nil {
			return fmt.Errorf("failed to move output: %w", err)
		}
	}

	return nil
}

// Close cleans up temporary files
func (c *KindleConverter) Close() error {
	if c.tempDir != "" {
		return os.RemoveAll(c.tempDir)
	}
	return nil
}

// Kindle-optimized HTML template for manga
const kindleMangaTemplate = `<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops">
<head>
    <title>{{.Title}}</title>
    <style>
        @page {
            margin: 0;
            padding: 0;
        }
        body {
            margin: 0;
            padding: 0;
            text-align: center;
            background-color: white;
        }
        .page {
            page-break-after: always;
            page-break-inside: avoid;
            margin: 0;
            padding: 0;
            height: 100vh;
            display: flex;
            align-items: center;
            justify-content: center;
        }
        img {
            max-width: 100%;
            max-height: 100vh;
            width: auto;
            height: auto;
            display: block;
            margin: 0 auto;
        }
        /* Kindle-specific optimizations */
        @media amzn-kf8 {
            .page {
                text-align: center;
            }
        }
        @media amzn-mobi {
            img {
                max-width: 100%;
            }
        }
    </style>
</head>
<body>
    {{range .Pages}}
    <div class="page">
        <img src="{{.Path}}" alt="Page {{.Index}}" />
    </div>
    {{end}}
</body>
</html>`

// Helper template structure
type KindleTemplateData struct {
	Title string
	Pages []PageData
}

// renderKindleHTML renders HTML optimized for Kindle devices
func (c *KindleConverter) renderKindleHTML(title string, pages []PageData) (string, error) {
	tmpl, err := template.New("kindle").Parse(kindleMangaTemplate)
	if err != nil {
		return "", err
	}

	data := KindleTemplateData{
		Title: title,
		Pages: pages,
	}

	var buf bytes.Buffer
	if err := tmpl.Execute(&buf, data); err != nil {
		return "", err
	}

	return buf.String(), nil
}
