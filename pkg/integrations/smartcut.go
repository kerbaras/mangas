package integrations

import (
	"fmt"
	"image"
	"image/color"
	"math"
	"runtime"
	"strings"
	"sync"

	"golang.org/x/image/draw"
)

// SmartCutOptions controls how long vertical strips (webtoons / manhwa) are split into device pages.
//
// Instead of cutting every N rows, the paginator scores every row of the strip (gutter rows are free,
// rows inside panels cost a little, rows inside text or speech bubbles cost a lot) and picks the cut
// rows that minimise page count plus cut damage by dynamic programming. Two lossless escape hatches
// are used when no clean cut exists: a page may be rendered up to MaxShrink smaller so a whole panel
// fits, and a cut through artwork makes the next page repeat the last rows (overlap).
type SmartCutOptions struct {
	DeviceWidth   int
	DeviceHeight  int
	MaxShrink     float64 // 1.10 = a page may hold 10% more rows than fit at native zoom; 1.0 = never shrink
	ShrinkPenalty float64 // cost of shrinking a page by 100% of capacity; lower trades zoom for fewer bad cuts
	OverlapFrac   float64 // rows repeated after a cut through artwork, as a fraction of page capacity
	AnalysisWidth int     // strips wider than this are downscaled for analysis only
}

// Smart-cut presets, named after the reading-experience trade-off they make.
const (
	// PresetBalanced shrinks a page by at most 10% and only when it avoids a costly cut.
	PresetBalanced = "balanced"
	// PresetFewestCuts allows up to 25% shrink and shrinks readily; fewest cuts through panels and bubbles.
	PresetFewestCuts = "fewest-cuts"
	// PresetNoShrink never shrinks; relies on overlap alone.
	PresetNoShrink = "no-shrink"
)

// SmartCutPresets lists the valid preset names.
var SmartCutPresets = []string{PresetBalanced, PresetFewestCuts, PresetNoShrink}

// DefaultSmartCutOptions returns the balanced preset for the device.
func DefaultSmartCutOptions(d KindleDevice) SmartCutOptions {
	o, _ := SmartCutPreset(PresetBalanced, d)
	return o
}

// SmartCutPreset returns the options for a named preset.
func SmartCutPreset(name string, d KindleDevice) (SmartCutOptions, error) {
	o := SmartCutOptions{
		DeviceWidth:   d.Width,
		DeviceHeight:  d.Height,
		OverlapFrac:   0.075,
		AnalysisWidth: 720,
	}
	switch name {
	case PresetBalanced, "":
		o.MaxShrink, o.ShrinkPenalty = 1.10, 30
	case PresetFewestCuts:
		o.MaxShrink, o.ShrinkPenalty = 1.25, 10
	case PresetNoShrink:
		o.MaxShrink, o.ShrinkPenalty = 1.0, 30
	default:
		return o, fmt.Errorf("unknown smart-cut preset %q (want one of %s)", name, strings.Join(SmartCutPresets, ", "))
	}
	return o, nil
}

const (
	cutCostBase       = 0.5
	cutCostPanel      = 1.0
	cutCostBubble     = 30.0
	cutCostText       = 60.0
	cutPageCost       = 1.0
	cutBadThreshold   = 0.01 // any cut that is not on a gutter row counts as "through art"
	longStripMinPages = 1.25 // an image is a strip if it needs more than this many pages
)

// IsLongStrip reports whether an image is a vertical strip that needs smart cutting for the device,
// as opposed to a regular page that is simply scaled to fit.
func IsLongStrip(w, h int, o SmartCutOptions) bool {
	if w <= 0 || h <= 0 {
		return false
	}
	return float64(h)/float64(w) > float64(o.DeviceHeight)/float64(o.DeviceWidth)*longStripMinPages
}

// pageSpan is a page as a half-open row range of the source strip.
type pageSpan struct {
	Start, End int
}

// cutCosts returns the cost of placing a page boundary just above each row (index H = end of strip, free).
func cutCosts(F *stripFeatures) []float64 {
	c := make([]float64, F.H+1)
	for y := 0; y < F.H; y++ {
		if F.IsGutter[y] {
			continue
		}
		c[y] = cutCostBase + float64(F.Texture[y]) + cutCostPanel*float64(F.RowPanel[y]) +
			cutCostBubble*float64(F.RowBubble[y]) + cutCostText*float64(F.RowText[y])
	}
	return c
}

// paginateRows chooses page boundaries for a strip of F.H rows where cap rows fit on a page at native zoom.
func paginateRows(F *stripFeatures, cap int, maxShrink, shrinkPenalty float64, overlap int) []pageSpan {
	H := F.H
	if H == 0 {
		return nil
	}
	cost := cutCosts(F)
	if !(maxShrink >= 1) { // never below native zoom; also guards NaN
		maxShrink = 1
	}
	maxH := int(float64(cap) * maxShrink)
	inf := math.Inf(1)
	best := make([]float64, H+1)
	prev := make([]int, H+1)
	for i := range best {
		best[i] = inf
		prev[i] = -1
	}
	best[0] = 0
	pageStart := func(a int) int {
		if a > 0 && cost[a] > cutBadThreshold {
			return maxInt(0, a-overlap)
		}
		return a
	}
	for a := 0; a < H; a++ {
		if math.IsInf(best[a], 1) {
			continue
		}
		start := pageStart(a)
		hi := minInt(H, start+maxH)
		for b := a + 1; b <= hi; b++ {
			h := b - start
			pen := 0.0
			if h > cap {
				pen = shrinkPenalty * float64(h-cap) / float64(cap)
			}
			tot := best[a] + cutPageCost + cost[b] + pen
			if tot < best[b] {
				best[b] = tot
				prev[b] = a
			}
		}
	}
	var pages []pageSpan
	for b := H; b > 0; {
		a := prev[b]
		pages = append(pages, pageSpan{pageStart(a), b})
		b = a
	}
	for i, j := 0, len(pages)-1; i < j; i, j = i+1, j-1 {
		pages[i], pages[j] = pages[j], pages[i]
	}
	return pages
}

// renderPage draws source rows [sp.Start, sp.End) of the strip onto a device-sized canvas: scaled to the
// device width (or a little smaller when the page was allowed to shrink), top-aligned, padded with bg.
func renderPage(src *image.RGBA, sp pageSpan, bg color.RGBA, devW, devH int) *image.RGBA {
	srcW := src.Bounds().Dx()
	srcH := sp.End - sp.Start
	dst := image.NewRGBA(image.Rect(0, 0, devW, devH))
	draw.Draw(dst, dst.Bounds(), image.NewUniform(bg), image.Point{}, draw.Src)
	if srcH <= 0 || srcW <= 0 {
		return dst
	}
	scale := float64(devW) / float64(srcW)
	outW, outH := devW, int(math.Round(float64(srcH)*scale))
	if outH > devH {
		scale = float64(devH) / float64(srcH)
		outW, outH = int(math.Round(float64(srcW)*scale)), devH
	}
	x0 := (devW - outW) / 2
	srcRect := image.Rect(src.Bounds().Min.X, src.Bounds().Min.Y+sp.Start, src.Bounds().Max.X, src.Bounds().Min.Y+sp.End)
	draw.CatmullRom.Scale(dst, image.Rect(x0, 0, x0+outW, outH), src, srcRect, draw.Src, nil)
	return dst
}

// StripPaginator turns a sequence of same-width strip images (one chapter, in reading order) into
// device pages. Chapters are usually delivered as arbitrary 10 000 px chunks, so the paginator carries
// the unfinished tail of each image over to the next one; a panel straddling two files is never cut
// at the file boundary.
type StripPaginator struct {
	opts  SmartCutOptions
	carry *image.RGBA
}

// NewStripPaginator creates a paginator for the given options.
func NewStripPaginator(o SmartCutOptions) *StripPaginator {
	return &StripPaginator{opts: o}
}

// Feed adds the next strip image and returns the pages that are final so far. When last is false the
// tail of the strip is held back until the next Feed (or Flush).
func (p *StripPaginator) Feed(img image.Image, last bool) []*image.RGBA {
	var out []*image.RGBA
	if p.carry != nil && p.carry.Bounds().Dx() != img.Bounds().Dx() {
		out = append(out, p.Flush()...)
	}
	unit := stackImages(p.carry, img)
	p.carry = nil
	pages, _, remainder := p.paginateUnit(unit, last)
	out = append(out, pages...)
	if remainder != nil {
		p.carry = remainder
	}
	return out
}

// HasPending reports whether rows are held back waiting for the next chunk of the same width.
func (p *StripPaginator) HasPending(width int) bool {
	return p.carry != nil && p.carry.Bounds().Dx() == width
}

// Flush paginates whatever is still held back and returns those pages.
func (p *StripPaginator) Flush() []*image.RGBA {
	if p.carry == nil {
		return nil
	}
	unit := p.carry
	p.carry = nil
	pages, _, _ := p.paginateUnit(unit, true)
	return pages
}

// paginateUnit returns the rendered pages, their source row spans, and the held-back remainder (nil when last).
func (p *StripPaginator) paginateUnit(unit *image.RGBA, last bool) ([]*image.RGBA, []pageSpan, *image.RGBA) {
	W, H := unit.Bounds().Dx(), unit.Bounds().Dy()
	if W == 0 || H == 0 {
		return nil, nil, nil
	}
	analysis := resizedForAnalysis(unit, p.opts.AnalysisWidth)
	s := newRGBStrip(analysis)
	F := computeStripFeatures(s)
	toSrc := float64(H) / float64(s.H)
	cap := int(math.Floor(float64(s.W) * float64(p.opts.DeviceHeight) / float64(p.opts.DeviceWidth)))
	if cap < 1 {
		cap = 1
	}
	overlap := int(math.Round(float64(cap) * p.opts.OverlapFrac))
	// overlap must leave room to advance: never more than half of what fits on a page
	overlap = clampInt(overlap, 0, cap/2)
	spans := paginateRows(F, cap, p.opts.MaxShrink, p.opts.ShrinkPenalty, overlap)
	if !last {
		// hold the unfinished tail back for the next chunk
		if len(spans) <= 1 {
			return nil, nil, unit
		}
		tail := spans[len(spans)-1]
		spans = spans[:len(spans)-1]
		start := clampInt(int(math.Round(float64(tail.Start)*toSrc)), 0, H-1)
		rem := image.NewRGBA(image.Rect(0, 0, W, H-start))
		draw.Draw(rem, rem.Bounds(), unit, image.Point{unit.Bounds().Min.X, unit.Bounds().Min.Y + start}, draw.Src)
		pages, src := p.render(unit, F, spans, toSrc)
		return pages, src, rem
	}
	pages, src := p.render(unit, F, spans, toSrc)
	return pages, src, nil
}

func (p *StripPaginator) render(unit *image.RGBA, F *stripFeatures, spans []pageSpan, toSrc float64) ([]*image.RGBA, []pageSpan) {
	H := unit.Bounds().Dy()
	src := make([]pageSpan, 0, len(spans))
	bgs := make([]color.RGBA, 0, len(spans))
	for _, sp := range spans {
		a := clampInt(int(math.Round(float64(sp.Start)*toSrc)), 0, H)
		b := clampInt(int(math.Round(float64(sp.End)*toSrc)), 0, H)
		if sp.End == F.H {
			b = H
		}
		if b <= a {
			continue
		}
		bg := F.BG[clampInt(sp.Start, 0, F.H-1)]
		src = append(src, pageSpan{a, b})
		bgs = append(bgs, color.RGBA{bg[0], bg[1], bg[2], 255})
	}
	out := make([]*image.RGBA, len(src))
	sem := make(chan struct{}, runtime.GOMAXPROCS(0))
	var wg sync.WaitGroup
	for i := range src {
		wg.Add(1)
		sem <- struct{}{}
		go func(i int) {
			defer wg.Done()
			defer func() { <-sem }()
			out[i] = renderPage(unit, src[i], bgs[i], p.opts.DeviceWidth, p.opts.DeviceHeight)
		}(i)
	}
	wg.Wait()
	return out, src
}

// stackImages returns top (may be nil) stacked above bottom as one RGBA image.
func stackImages(top *image.RGBA, bottom image.Image) *image.RGBA {
	bb := bottom.Bounds()
	topH := 0
	if top != nil {
		topH = top.Bounds().Dy()
	}
	dst := image.NewRGBA(image.Rect(0, 0, bb.Dx(), topH+bb.Dy()))
	if top != nil {
		draw.Draw(dst, image.Rect(0, 0, bb.Dx(), topH), top, top.Bounds().Min, draw.Src)
	}
	draw.Draw(dst, image.Rect(0, topH, bb.Dx(), topH+bb.Dy()), bottom, bb.Min, draw.Src)
	return dst
}
