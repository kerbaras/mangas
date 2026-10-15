package integrations

import (
	"bytes"
	"fmt"
	"image"
	"image/jpeg"
	"image/png"
	"io"
	"math"

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
	img, format, err := decodeImageFrom(input)
	if err != nil {
		return nil, fmt.Errorf("failed to decode image: %w", err)
	}
	return p.processDecoded(img, format)
}

// ProcessDecoded optimizes an already decoded image for Kindle display
func (p *ImageProcessor) ProcessDecoded(img image.Image) ([]byte, error) {
	return p.processDecoded(img, "")
}

func (p *ImageProcessor) processDecoded(img image.Image, format string) ([]byte, error) {
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

	// Apply contrast and gamma in one pass if needed
	if p.settings.Contrast != 1.0 || p.settings.Gamma != 1.0 {
		processed = applyLUT(processed, p.toneLUT())
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

// toRGBA returns img as a tightly packed, zero-origin *image.RGBA, without copying when it already is one.
func toRGBA(img image.Image) *image.RGBA {
	if rgba, ok := img.(*image.RGBA); ok && isPacked(rgba.Bounds(), rgba.Stride, 4) {
		return rgba
	}
	b := img.Bounds()
	dst := image.NewRGBA(image.Rect(0, 0, b.Dx(), b.Dy()))
	draw.Draw(dst, dst.Bounds(), img, b.Min, draw.Src)
	return dst
}

// isPacked reports whether an image with these bounds and stride is zero-origin with no row padding,
// so its Pix slice can be walked linearly.
func isPacked(b image.Rectangle, stride, channels int) bool {
	return b.Min == image.Point{} && stride == b.Dx()*channels
}

// toGray returns img as a tightly packed, zero-origin *image.Gray (copying only sub-images), or nil.
func toGray(img image.Image) *image.Gray {
	g, ok := img.(*image.Gray)
	if !ok {
		return nil
	}
	if isPacked(g.Bounds(), g.Stride, 1) {
		return g
	}
	b := g.Bounds()
	dst := image.NewGray(image.Rect(0, 0, b.Dx(), b.Dy()))
	draw.Draw(dst, dst.Bounds(), g, b.Min, draw.Src)
	return dst
}

// toGrayscale converts an image to grayscale (BT.601 luma)
func (p *ImageProcessor) toGrayscale(img image.Image) image.Image {
	if g := toGray(img); g != nil {
		return g
	}
	src := toRGBA(img)
	w, h := src.Bounds().Dx(), src.Bounds().Dy()
	gray := image.NewGray(image.Rect(0, 0, w, h))
	for y := 0; y < h; y++ {
		s := src.Pix[y*src.Stride : y*src.Stride+w*4]
		d := gray.Pix[y*gray.Stride : y*gray.Stride+w]
		for x := 0; x < w; x++ {
			d[x] = uint8((19595*uint32(s[x*4]) + 38470*uint32(s[x*4+1]) + 7471*uint32(s[x*4+2]) + 1<<15) >> 16)
		}
	}
	return gray
}

// adjustContrast adjusts the contrast of an image
func (p *ImageProcessor) adjustContrast(img image.Image, factor float64) image.Image {
	var lut [256]uint8
	for i := range lut {
		lut[i] = p.adjustChannel(uint8(i), factor)
	}
	return applyLUT(img, &lut)
}

// adjustChannel adjusts a single color channel
func (p *ImageProcessor) adjustChannel(value uint8, factor float64) uint8 {
	// Center around 128 (middle gray)
	adjusted := (float64(value)-128)*factor + 128

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
	var lut [256]uint8
	for i := range lut {
		lut[i] = gammaChannel(uint8(i), gamma)
	}
	return applyLUT(img, &lut)
}

func gammaChannel(value uint8, gamma float64) uint8 {
	corrected := 255.0 * math.Pow(float64(value)/255.0, 1.0/gamma)
	if corrected > 255 {
		corrected = 255
	}
	return uint8(corrected + 0.5)
}

// toneLUT folds the contrast and gamma steps into one lookup table.
func (p *ImageProcessor) toneLUT() *[256]uint8 {
	var lut [256]uint8
	for i := range lut {
		v := uint8(i)
		if p.settings.Contrast != 1.0 {
			v = p.adjustChannel(v, p.settings.Contrast)
		}
		if p.settings.Gamma != 1.0 {
			v = gammaChannel(v, p.settings.Gamma)
		}
		lut[i] = v
	}
	return &lut
}

// applyLUT maps every colour channel through lut (alpha untouched), keeping the image type.
func applyLUT(img image.Image, lut *[256]uint8) image.Image {
	if src := toGray(img); src != nil {
		dst := image.NewGray(src.Bounds())
		for i, v := range src.Pix {
			dst.Pix[i] = lut[v]
		}
		return dst
	}
	rgba := toRGBA(img)
	dst := image.NewRGBA(rgba.Bounds())
	for i, v := range rgba.Pix {
		if i&3 == 3 {
			dst.Pix[i] = v
		} else {
			dst.Pix[i] = lut[v]
		}
	}
	return dst
}

// sharpen applies a simple sharpening filter for e-ink displays
//
// 3x3 kernel [-1 -1 -1; -1 9 -1; -1 -1 -1]; border pixels are copied as-is.
func (p *ImageProcessor) sharpen(img image.Image) image.Image {
	if src := toGray(img); src != nil {
		dst := image.NewGray(src.Bounds())
		copy(dst.Pix, src.Pix)
		sharpenPix(src.Pix, dst.Pix, src.Stride, 1, src.Bounds().Dx(), src.Bounds().Dy())
		return dst
	}
	rgba := toRGBA(img)
	dst := image.NewRGBA(rgba.Bounds())
	copy(dst.Pix, rgba.Pix)
	sharpenPix(rgba.Pix, dst.Pix, rgba.Stride, 4, rgba.Bounds().Dx(), rgba.Bounds().Dy())
	return dst
}

func sharpenPix(src, dst []uint8, stride, channels, w, h int) {
	colour := channels
	if channels == 4 {
		colour = 3 // leave alpha alone
	}
	for y := 1; y < h-1; y++ {
		up := src[(y-1)*stride:]
		mid := src[y*stride:]
		down := src[(y+1)*stride:]
		row := dst[y*stride:]
		for x := 1; x < w-1; x++ {
			for c := 0; c < colour; c++ {
				i := x*channels + c
				sum := 9*int32(mid[i]) -
					int32(up[i-channels]) - int32(up[i]) - int32(up[i+channels]) -
					int32(mid[i-channels]) - int32(mid[i+channels]) -
					int32(down[i-channels]) - int32(down[i]) - int32(down[i+channels])
				row[i] = clamp(sum)
			}
		}
	}
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
