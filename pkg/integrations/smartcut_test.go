package integrations

import (
	"image"
	"image/color"
	"image/draw"
	"testing"
)

// syntheticStrip paints black rectangles (panels) on a white strip. Each panel is given as [y0, y1).
func syntheticStrip(w, h int, panels [][2]int) *image.RGBA {
	img := image.NewRGBA(image.Rect(0, 0, w, h))
	draw.Draw(img, img.Bounds(), image.NewUniform(color.White), image.Point{}, draw.Src)
	for _, p := range panels {
		draw.Draw(img, image.Rect(20, p[0], w-20, p[1]), image.NewUniform(color.RGBA{30, 30, 30, 255}), image.Point{}, draw.Src)
	}
	return img
}

func TestLabelComponents(t *testing.T) {
	w, h := 10, 6
	m := make([]uint8, w*h)
	set := func(x, y int) { m[y*w+x] = 1 }
	set(1, 1)
	set(2, 1)
	set(1, 2) // component A: 3 px
	set(5, 1)
	set(6, 2) // component B: diagonal touch -> one component with 8-connectivity
	set(8, 5) // component C
	_, comps := labelComponents(m, w, h)
	if len(comps) != 3 {
		t.Fatalf("want 3 components, got %d: %+v", len(comps), comps)
	}
	var a *component
	for i := range comps {
		if comps[i].Area == 3 {
			a = &comps[i]
		}
	}
	if a == nil || a.X0 != 1 || a.Y0 != 1 || a.X1 != 3 || a.Y1 != 3 {
		t.Fatalf("component A box wrong: %+v", comps)
	}
}

func TestComputeStripFeatures_GuttersAndPanels(t *testing.T) {
	img := syntheticStrip(400, 1200, [][2]int{{50, 450}, {520, 900}, {980, 1150}})
	F := computeStripFeatures(newRGBStrip(img))
	if len(F.Gutters) != 4 {
		t.Fatalf("want 4 gutters (top, 2 inner, bottom), got %d: %+v", len(F.Gutters), F.Gutters)
	}
	if !F.IsGutter[480] || F.IsGutter[300] {
		t.Fatal("gutter mask wrong")
	}
	if F.BG[300] != [3]uint8{255, 255, 255} {
		t.Fatalf("background should be white, got %v", F.BG[300])
	}
	if len(F.Panels) != 3 {
		t.Fatalf("want 3 panels, got %d: %+v", len(F.Panels), F.Panels)
	}
	if F.RowPanel[300] != 1 || F.RowPanel[480] != 0 {
		t.Fatal("row panel coverage wrong")
	}
	if len(F.Text) != 0 {
		t.Fatalf("flat rectangles must not be detected as text: %+v", F.Text)
	}
}

func TestComputeStripFeatures_DarkBackground(t *testing.T) {
	img := image.NewRGBA(image.Rect(0, 0, 300, 600))
	draw.Draw(img, img.Bounds(), image.NewUniform(color.RGBA{20, 22, 30, 255}), image.Point{}, draw.Src)
	draw.Draw(img, image.Rect(10, 100, 290, 300), image.NewUniform(color.RGBA{200, 180, 160, 255}), image.Point{}, draw.Src)
	F := computeStripFeatures(newRGBStrip(img))
	if F.BG[200] != [3]uint8{20, 22, 30} {
		t.Fatalf("dark gutter colour should be the background, got %v", F.BG[200])
	}
	if len(F.Panels) != 1 || F.Panels[0].Y0 != 100 || F.Panels[0].Y1 != 300 {
		t.Fatalf("panel on dark background not found: %+v", F.Panels)
	}
}

func featuresFromRows(h int, gutter func(y int) bool, text func(y int) bool) *stripFeatures {
	F := &stripFeatures{H: h, W: 100, IsGutter: make([]bool, h), BG: make([][3]uint8, h),
		Texture: make([]float32, h), RowPanel: make([]float32, h), RowText: make([]float32, h), RowBubble: make([]float32, h)}
	for y := 0; y < h; y++ {
		F.Texture[y] = 1
		if gutter(y) {
			F.IsGutter[y] = true
		} else {
			F.RowPanel[y] = 1
		}
		if text != nil && text(y) {
			F.RowText[y] = 1
			F.RowBubble[y] = 1
		}
	}
	return F
}

func checkCoverage(t *testing.T, pages []pageSpan, h, maxH int) {
	t.Helper()
	if len(pages) == 0 || pages[0].Start != 0 || pages[len(pages)-1].End != h {
		t.Fatalf("pages must span the whole strip: %+v", pages)
	}
	for i, p := range pages {
		if p.End-p.Start > maxH {
			t.Fatalf("page %d taller than allowed (%d > %d)", i, p.End-p.Start, maxH)
		}
		if i > 0 && pages[i-1].End < p.Start {
			t.Fatalf("rows %d-%d lost between pages", pages[i-1].End, p.Start)
		}
	}
}

func TestPaginateRows_CutsOnGutters(t *testing.T) {
	// panels of 700 rows separated by 40-row gutters; capacity 900 -> one panel per page
	h := 5 * 740
	gutter := func(y int) bool { return y%740 >= 700 }
	F := featuresFromRows(h, gutter, nil)
	pages := paginateRows(F, 900, 1.10, 30, 60)
	checkCoverage(t, pages, h, 990)
	if len(pages) != 5 {
		t.Fatalf("want 5 pages, got %d: %+v", len(pages), pages)
	}
	for _, p := range pages[:len(pages)-1] {
		if !gutter(p.End) {
			t.Fatalf("cut at %d is not on a gutter: %+v", p.End, pages)
		}
	}
}

func TestPaginateRows_ShrinkToFitPanel(t *testing.T) {
	// a 950-row panel with capacity 900: shrink (<=10%) must be preferred over cutting it
	h := 950 + 40 + 500
	gutter := func(y int) bool { return y >= 950 && y < 990 }
	F := featuresFromRows(h, gutter, nil)
	pages := paginateRows(F, 900, 1.10, 30, 60)
	checkCoverage(t, pages, h, 990)
	if len(pages) != 2 || pages[0].End < 950 || pages[0].End > 990 {
		t.Fatalf("expected the first page to hold the whole panel: %+v", pages)
	}
}

func TestPaginateRows_AvoidsTextAndOverlaps(t *testing.T) {
	// no gutters at all: a 3000-row full-bleed panel with text at rows 850-1000
	h := 3000
	F := featuresFromRows(h, func(int) bool { return false }, func(y int) bool { return y >= 850 && y < 1000 })
	pages := paginateRows(F, 900, 1.10, 30, 60)
	checkCoverage(t, pages, h, 990)
	for i, p := range pages {
		if p.End < h && p.End >= 850 && p.End < 1000 {
			t.Fatalf("cut %d goes through text: %+v", p.End, pages)
		}
		if i > 0 && pages[i-1].End-p.Start != 60 {
			t.Fatalf("cut through artwork must overlap by 60 rows: %+v", pages)
		}
	}
}

func TestIsLongStrip(t *testing.T) {
	o := DefaultSmartCutOptions(KindleDevices["kindle-paperwhite12"])
	if !IsLongStrip(709, 10000, o) {
		t.Error("webtoon chunk should be a long strip")
	}
	if IsLongStrip(1600, 2400, o) {
		t.Error("a manga page must not be treated as a strip")
	}
	if IsLongStrip(0, 0, o) {
		t.Error("empty image")
	}
}

func TestRenderPage(t *testing.T) {
	src := syntheticStrip(400, 2000, [][2]int{{0, 2000}})
	bg := color.RGBA{255, 0, 0, 255}
	// native fit: 400 rows -> 600 device rows, rest letterboxed with bg
	pg := renderPage(src, pageSpan{0, 400}, bg, 600, 800)
	if pg.Bounds().Dx() != 600 || pg.Bounds().Dy() != 800 {
		t.Fatalf("page size %v", pg.Bounds())
	}
	if pg.RGBAAt(300, 790) != bg {
		t.Fatalf("letterbox should use the background colour, got %v", pg.RGBAAt(300, 790))
	}
	if c := pg.RGBAAt(300, 300); c.R > 60 {
		t.Fatalf("content should be drawn, got %v", c)
	}
	// shrunk page: 580 rows do not fit at native zoom -> scaled down and centred
	pg = renderPage(src, pageSpan{0, 580}, bg, 600, 800)
	if pg.RGBAAt(2, 400) != bg || pg.RGBAAt(597, 400) != bg {
		t.Fatal("shrunk page should be centred with bg margins")
	}
	if c := pg.RGBAAt(300, 795); c.R > 60 {
		t.Fatalf("shrunk page should fill the full height, got %v", c)
	}
}

func TestStripPaginator_CarryOverAcrossFiles(t *testing.T) {
	o := SmartCutOptions{DeviceWidth: 600, DeviceHeight: 800, MaxShrink: 1.10, ShrinkPenalty: 30, OverlapFrac: 0.075, AnalysisWidth: 720}
	// capacity for a 400-wide strip is 533 rows. A 500-row panel straddles the file boundary at 1200.
	panels := [][2]int{{20, 480}, {520, 980}, {1020, 1520}, {1560, 2000}, {2040, 2380}}
	whole := syntheticStrip(400, 2400, panels)
	first := image.NewRGBA(image.Rect(0, 0, 400, 1200))
	draw.Draw(first, first.Bounds(), whole, image.Point{}, draw.Src)
	second := image.NewRGBA(image.Rect(0, 0, 400, 1200))
	draw.Draw(second, second.Bounds(), whole, image.Point{0, 1200}, draw.Src)

	p := NewStripPaginator(o)
	var spans []pageSpan
	unit := stackImages(nil, first)
	pages, src, rem := p.paginateUnit(unit, false)
	if rem == nil {
		t.Fatal("first chunk should hold back its tail")
	}
	spans = append(spans, src...)
	offset := 1200 - rem.Bounds().Dy()
	unit = stackImages(rem, second)
	pages2, src2, rem2 := p.paginateUnit(unit, true)
	if rem2 != nil {
		t.Fatal("last chunk must not hold anything back")
	}
	for _, s := range src2 {
		spans = append(spans, pageSpan{s.Start + offset, s.End + offset})
	}
	checkCoverage(t, spans, 2400, 587)
	if len(pages)+len(pages2) != len(spans) {
		t.Fatal("rendered page count mismatch")
	}
	for _, s := range spans[:len(spans)-1] {
		for _, pn := range panels {
			if s.End > pn[0] && s.End < pn[1] {
				t.Fatalf("cut at %d splits panel %v (spans %+v)", s.End, pn, spans)
			}
		}
	}

	// the public API yields the same number of pages
	q := NewStripPaginator(o)
	n := len(q.Feed(first, false)) + len(q.Feed(second, true))
	if n != len(spans) {
		t.Fatalf("Feed produced %d pages, want %d", n, len(spans))
	}
}

func TestStripPaginator_FlushAndWidthChange(t *testing.T) {
	o := SmartCutOptions{DeviceWidth: 600, DeviceHeight: 800, MaxShrink: 1.10, ShrinkPenalty: 30, OverlapFrac: 0.075, AnalysisWidth: 720}
	p := NewStripPaginator(o)
	if got := p.Feed(syntheticStrip(400, 300, [][2]int{{20, 280}}), false); len(got) != 0 {
		t.Fatalf("a short non-final chunk should be held back, got %d pages", len(got))
	}
	// a chunk of a different width flushes the held-back rows as their own page first
	got := p.Feed(syntheticStrip(500, 1600, [][2]int{{20, 700}, {760, 1500}}), true)
	if len(got) < 3 {
		t.Fatalf("want held-back page + pages of the new strip, got %d", len(got))
	}
	if p.Flush() != nil {
		t.Fatal("nothing should remain after a final chunk")
	}
}

func TestSmartCutPreset(t *testing.T) {
	d := KindleDevices["kindle-paperwhite12"]
	for _, name := range SmartCutPresets {
		o, err := SmartCutPreset(name, d)
		if err != nil || o.DeviceWidth != d.Width || o.MaxShrink < 1.0 {
			t.Fatalf("preset %s: %+v %v", name, o, err)
		}
	}
	if _, err := SmartCutPreset("bogus", d); err == nil {
		t.Fatal("unknown preset must error")
	}
	if DefaultSmartCutOptions(d).MaxShrink != 1.10 {
		t.Fatal("default preset should be balanced")
	}
}

func TestFilters_SubImage(t *testing.T) {
	proc := NewImageProcessor(ImageOptimizationSettings{MaxWidth: 1000, MaxHeight: 1000, Quality: 80, Grayscale: true, Sharpen: true, Contrast: 1.1, Gamma: 0.9, Format: "jpeg"})
	full := image.NewGray(image.Rect(0, 0, 200, 200))
	for i := range full.Pix {
		full.Pix[i] = uint8(i)
	}
	sub := full.SubImage(image.Rect(50, 50, 150, 120)).(*image.Gray)
	out, err := proc.ProcessDecoded(sub)
	if err != nil || len(out) == 0 {
		t.Fatalf("gray sub-image: %v", err)
	}
	rgba := image.NewRGBA(image.Rect(0, 0, 200, 200))
	subRGBA := rgba.SubImage(image.Rect(10, 20, 90, 70)).(*image.RGBA)
	if g := proc.toGrayscale(subRGBA); g.Bounds().Dx() != 80 || g.Bounds().Dy() != 50 {
		t.Fatalf("rgba sub-image: %v", g.Bounds())
	}
	if _, err := proc.ProcessDecoded(subRGBA); err != nil {
		t.Fatal(err)
	}
}

func TestStripPaginator_ShortTrailingChunkStaysInStrip(t *testing.T) {
	o := SmartCutOptions{DeviceWidth: 600, DeviceHeight: 800, MaxShrink: 1.10, ShrinkPenalty: 30, OverlapFrac: 0.075, AnalysisWidth: 720}
	p := NewStripPaginator(o)
	p.Feed(syntheticStrip(400, 1500, [][2]int{{20, 500}, {540, 1000}, {1040, 1500}}), false)
	if !p.HasPending(400) || p.HasPending(500) {
		t.Fatal("pending rows should be reported for the same width only")
	}
	// a 300-row tail is not a long strip on its own but must join the held-back rows
	got := p.Feed(syntheticStrip(400, 300, [][2]int{{0, 260}}), true)
	if len(got) == 0 || p.HasPending(400) {
		t.Fatalf("tail should have been paginated with the pending rows, got %d pages", len(got))
	}
}
