package integrations

import (
	"bytes"
	"fmt"
	"image"
	"image/color"
	"image/jpeg"
	"image/png"
	"io"

	"golang.org/x/image/draw"
)

// ImageProcessor handles image optimization for Kindle devices
type ImageProcessor struct {
	settings ImageOptimizationSettings
}

// NewImageProcessor creates a new image processor with the given settings
func NewImageProcessor(settings ImageOptimizationSettings) *ImageProcessor {
	return &ImageProcessor{
		settings: settings,
	}
}

// ProcessImage optimizes an image for Kindle display
func (p *ImageProcessor) ProcessImage(input io.Reader) ([]byte, error) {
	// Decode image
	img, format, err := image.Decode(input)
	if err != nil {
		return nil, fmt.Errorf("failed to decode image: %w", err)
	}

	// Get original dimensions
	bounds := img.Bounds()
	origWidth := bounds.Dx()
	origHeight := bounds.Dy()

	// Calculate new dimensions while maintaining aspect ratio
	newWidth, newHeight := p.calculateDimensions(origWidth, origHeight)

	// Resize if needed
	var processed image.Image = img
	if newWidth != origWidth || newHeight != origHeight {
		processed = p.resize(img, newWidth, newHeight)
	}

	// Convert to grayscale if needed
	if p.settings.Grayscale && format != "gray" {
		processed = p.toGrayscale(processed)
	}

	// Apply contrast adjustment if needed
	if p.settings.Contrast != 1.0 {
		processed = p.adjustContrast(processed, p.settings.Contrast)
	}

	// Apply gamma correction if needed
	if p.settings.Gamma != 1.0 {
		processed = p.adjustGamma(processed, p.settings.Gamma)
	}

	// Apply sharpening for e-ink if enabled
	if p.settings.Sharpen {
		processed = p.sharpen(processed)
	}

	// Encode to output format
	return p.encode(processed)
}

// calculateDimensions calculates the new dimensions while maintaining aspect ratio
func (p *ImageProcessor) calculateDimensions(width, height int) (int, int) {
	if width <= p.settings.MaxWidth && height <= p.settings.MaxHeight {
		return width, height // No resize needed
	}

	// Calculate scaling factors
	widthScale := float64(p.settings.MaxWidth) / float64(width)
	heightScale := float64(p.settings.MaxHeight) / float64(height)

	// Use the smaller scale to ensure image fits within bounds
	scale := widthScale
	if heightScale < widthScale {
		scale = heightScale
	}

	newWidth := int(float64(width) * scale)
	newHeight := int(float64(height) * scale)

	return newWidth, newHeight
}

// resize resizes an image using high-quality interpolation
func (p *ImageProcessor) resize(img image.Image, width, height int) image.Image {
	dst := image.NewRGBA(image.Rect(0, 0, width, height))
	
	// Use CatmullRom for high-quality downscaling
	draw.CatmullRom.Scale(dst, dst.Bounds(), img, img.Bounds(), draw.Over, nil)
	
	return dst
}

// toGrayscale converts an image to grayscale
func (p *ImageProcessor) toGrayscale(img image.Image) image.Image {
	bounds := img.Bounds()
	gray := image.NewGray(bounds)

	for y := bounds.Min.Y; y < bounds.Max.Y; y++ {
		for x := bounds.Min.X; x < bounds.Max.X; x++ {
			gray.Set(x, y, img.At(x, y))
		}
	}

	return gray
}

// adjustContrast adjusts the contrast of an image
func (p *ImageProcessor) adjustContrast(img image.Image, factor float64) image.Image {
	bounds := img.Bounds()
	adjusted := image.NewRGBA(bounds)

	for y := bounds.Min.Y; y < bounds.Max.Y; y++ {
		for x := bounds.Min.X; x < bounds.Max.X; x++ {
			r, g, b, a := img.At(x, y).RGBA()
			
			// Convert to 0-255 range
			r8 := uint8(r >> 8)
			g8 := uint8(g >> 8)
			b8 := uint8(b >> 8)
			a8 := uint8(a >> 8)

			// Apply contrast adjustment
			r8 = p.adjustChannel(r8, factor)
			g8 = p.adjustChannel(g8, factor)
			b8 = p.adjustChannel(b8, factor)

			adjusted.SetRGBA(x, y, color.RGBA{r8, g8, b8, a8})
		}
	}

	return adjusted
}

// adjustChannel adjusts a single color channel
func (p *ImageProcessor) adjustChannel(value uint8, factor float64) uint8 {
	// Center around 128 (middle gray)
	adjusted := float64(value-128)*factor + 128
	
	if adjusted < 0 {
		return 0
	}
	if adjusted > 255 {
		return 255
	}
	
	return uint8(adjusted)
}

// adjustGamma applies gamma correction to an image
func (p *ImageProcessor) adjustGamma(img image.Image, gamma float64) image.Image {
	bounds := img.Bounds()
	adjusted := image.NewRGBA(bounds)

	// Build gamma lookup table
	gammaTable := make([]uint8, 256)
	for i := 0; i < 256; i++ {
		normalized := float64(i) / 255.0
		corrected := 255.0 * pow(normalized, 1.0/gamma)
		if corrected > 255 {
			corrected = 255
		}
		gammaTable[i] = uint8(corrected)
	}

	for y := bounds.Min.Y; y < bounds.Max.Y; y++ {
		for x := bounds.Min.X; x < bounds.Max.X; x++ {
			r, g, b, a := img.At(x, y).RGBA()
			
			r8 := gammaTable[uint8(r>>8)]
			g8 := gammaTable[uint8(g>>8)]
			b8 := gammaTable[uint8(b>>8)]
			a8 := uint8(a >> 8)

			adjusted.SetRGBA(x, y, color.RGBA{r8, g8, b8, a8})
		}
	}

	return adjusted
}

// pow is a simple power function for gamma calculation
func pow(x, y float64) float64 {
	if y == 1.0 {
		return x
	}
	if y == 2.0 {
		return x * x
	}
	// For other values, use a simple approximation
	// This is sufficient for gamma correction
	result := 1.0
	absY := y
	if y < 0 {
		absY = -y
	}
	
	for i := 0; i < int(absY*10); i++ {
		result *= x
	}
	
	if y < 0 {
		return 1.0 / result
	}
	return result
}

// sharpen applies a simple sharpening filter for e-ink displays
func (p *ImageProcessor) sharpen(img image.Image) image.Image {
	bounds := img.Bounds()
	sharpened := image.NewRGBA(bounds)

	// Simple 3x3 sharpening kernel
	// [ -1 -1 -1 ]
	// [ -1  9 -1 ]
	// [ -1 -1 -1 ]
	
	for y := bounds.Min.Y + 1; y < bounds.Max.Y-1; y++ {
		for x := bounds.Min.X + 1; x < bounds.Max.X-1; x++ {
			// Get surrounding pixels
			var rSum, gSum, bSum int32
			
			// Center pixel (weight: 9)
			r, g, b, a := img.At(x, y).RGBA()
			rSum += int32(r>>8) * 9
			gSum += int32(g>>8) * 9
			bSum += int32(b>>8) * 9

			// Surrounding pixels (weight: -1 each)
			for dy := -1; dy <= 1; dy++ {
				for dx := -1; dx <= 1; dx++ {
					if dx == 0 && dy == 0 {
						continue
					}
					r, g, b, _ := img.At(x+dx, y+dy).RGBA()
					rSum -= int32(r >> 8)
					gSum -= int32(g >> 8)
					bSum -= int32(b >> 8)
				}
			}

			// Clamp values
			r8 := clamp(rSum)
			g8 := clamp(gSum)
			b8 := clamp(bSum)
			a8 := uint8(a >> 8)

			sharpened.SetRGBA(x, y, color.RGBA{r8, g8, b8, a8})
		}
	}

	// Copy edges as-is
	for y := bounds.Min.Y; y < bounds.Max.Y; y++ {
		sharpened.Set(bounds.Min.X, y, img.At(bounds.Min.X, y))
		sharpened.Set(bounds.Max.X-1, y, img.At(bounds.Max.X-1, y))
	}
	for x := bounds.Min.X; x < bounds.Max.X; x++ {
		sharpened.Set(x, bounds.Min.Y, img.At(x, bounds.Min.Y))
		sharpened.Set(x, bounds.Max.Y-1, img.At(x, bounds.Max.Y-1))
	}

	return sharpened
}

// clamp restricts a value to the 0-255 range
func clamp(value int32) uint8 {
	if value < 0 {
		return 0
	}
	if value > 255 {
		return 255
	}
	return uint8(value)
}

// encode encodes the processed image to the specified format
func (p *ImageProcessor) encode(img image.Image) ([]byte, error) {
	var buf bytes.Buffer

	switch p.settings.Format {
	case "jpeg", "jpg":
		opts := &jpeg.Options{
			Quality: p.settings.Quality,
		}
		if err := jpeg.Encode(&buf, img, opts); err != nil {
			return nil, fmt.Errorf("failed to encode JPEG: %w", err)
		}
	case "png":
		if err := png.Encode(&buf, img); err != nil {
			return nil, fmt.Errorf("failed to encode PNG: %w", err)
		}
	default:
		return nil, fmt.Errorf("unsupported format: %s", p.settings.Format)
	}

	return buf.Bytes(), nil
}

// ProcessImageData is a convenience method that works with byte slices
func (p *ImageProcessor) ProcessImageData(data []byte) ([]byte, error) {
	return p.ProcessImage(bytes.NewReader(data))
}
