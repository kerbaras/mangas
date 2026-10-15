package integrations

import (
	"bytes"
	"image"
	"image/color"

	_ "golang.org/x/image/webp" // register WebP, the format most webtoon sources deliver
)

// decodeImage decodes JPEG, PNG, GIF or WebP data.
//
// Lossy WebP decodes to a YCbCr image whose planes use studio swing (Y in 16..235), while Go's
// colour conversion assumes full range, so blacks come out as 16 and whites as 235. libwebp (and
// therefore every viewer) applies the limited-range transform; do the same so background detection
// sees the real colours.
func decodeImage(data []byte) (image.Image, string, error) {
	img, format, err := image.Decode(bytes.NewReader(data))
	if err != nil {
		return nil, "", err
	}
	if ycc, ok := img.(*image.YCbCr); ok && format == "webp" {
		return limitedRangeYCbCrToRGBA(ycc), format, nil
	}
	return img, format, nil
}

func limitedRangeYCbCrToRGBA(src *image.YCbCr) *image.RGBA {
	b := src.Bounds()
	dst := image.NewRGBA(image.Rect(0, 0, b.Dx(), b.Dy()))
	for y := b.Min.Y; y < b.Max.Y; y++ {
		row := dst.Pix[(y-b.Min.Y)*dst.Stride:]
		for x := b.Min.X; x < b.Max.X; x++ {
			yy := int32(src.Y[src.YOffset(x, y)])
			ci := src.COffset(x, y)
			cb := int32(src.Cb[ci]) - 128
			cr := int32(src.Cr[ci]) - 128
			// BT.601 limited range, fixed point with 16 fractional bits
			yv := (yy - 16) * 76309                       // 1.164
			r := (yv + 104597*cr + 1<<15) >> 16           // 1.596 Cr
			g := (yv - 25675*cb - 53279*cr + 1<<15) >> 16 // 0.392 Cb, 0.813 Cr
			bl := (yv + 132201*cb + 1<<15) >> 16          // 2.017 Cb
			i := (x - b.Min.X) * 4
			row[i] = clamp(r)
			row[i+1] = clamp(g)
			row[i+2] = clamp(bl)
			row[i+3] = 255
		}
	}
	return dst
}

var _ = color.RGBA{}
