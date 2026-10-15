package integrations

import (
	"image"
	"math"
	"sort"

	"golang.org/x/image/draw"
)

// rgbStrip is a packed 8-bit RGB raster used by the smart-cut analysis.
type rgbStrip struct {
	W, H int
	Pix  []uint8 // len W*H*3, row-major
}

func newRGBStrip(img image.Image) *rgbStrip {
	b := img.Bounds()
	w, h := b.Dx(), b.Dy()
	s := &rgbStrip{W: w, H: h, Pix: make([]uint8, w*h*3)}
	rgba, ok := img.(*image.RGBA)
	if !ok {
		rgba = image.NewRGBA(image.Rect(0, 0, w, h))
		draw.Draw(rgba, rgba.Bounds(), img, b.Min, draw.Src)
	}
	for y := 0; y < h; y++ {
		src := rgba.Pix[y*rgba.Stride : y*rgba.Stride+w*4]
		dst := s.Pix[y*w*3 : (y+1)*w*3]
		for x := 0; x < w; x++ {
			dst[x*3], dst[x*3+1], dst[x*3+2] = src[x*4], src[x*4+1], src[x*4+2]
		}
	}
	return s
}

// resizedForAnalysis returns a copy no wider than maxW (aspect preserved).
func resizedForAnalysis(img image.Image, maxW int) image.Image {
	b := img.Bounds()
	if b.Dx() <= maxW {
		return img
	}
	h := int(math.Round(float64(b.Dy()) * float64(maxW) / float64(b.Dx())))
	if h < 1 {
		h = 1
	}
	dst := image.NewRGBA(image.Rect(0, 0, maxW, h))
	draw.CatmullRom.Scale(dst, dst.Bounds(), img, b, draw.Src, nil)
	return dst
}

type box struct{ X0, Y0, X1, Y1 int }

// rowSpan is a half-open [Start, End) range of rows.
type rowSpan struct{ Start, End int }

// stripFeatures is everything the paginator needs, one value per row.
type stripFeatures struct {
	H, W      int
	IsGutter  []bool
	BG        [][3]uint8 // background colour per row
	Texture   []float32  // mean |horizontal gradient| per row, normalised to mean 1
	RowPanel  []float32  // 1 if the row crosses a panel / floating object box
	RowText   []float32  // 1 if the row crosses a text block
	RowBubble []float32  // 1 if the row crosses a speech bubble
	Gutters   []rowSpan
	Panels    []box
	Text      []box
	Bubbles   []box
}

const (
	gutterStdThreshold = 3.0
	gutterMinRows      = 6
	fgTolerance        = 18
	panelCloseKernel   = 15
	panelMinArea       = 1500
	textContrastThr    = 25
	textBlurRadius     = 15 // 31x31 box blur
)

func computeStripFeatures(s *rgbStrip) *stripFeatures {
	F := &stripFeatures{H: s.H, W: s.W}
	uni := rowUniformity(s)
	F.Gutters = findGutters(uni)
	F.IsGutter = make([]bool, s.H)
	for _, g := range F.Gutters {
		for y := g.Start; y < g.End; y++ {
			F.IsGutter[y] = true
		}
	}
	F.BG = backgroundPerRow(s, F.Gutters)
	fg := foregroundMask(s, F.BG)
	F.Panels = panelBoxes(fg, s.W, s.H)
	F.Text = textBoxes(s)
	F.Bubbles = make([]box, len(F.Text))
	for i, t := range F.Text {
		F.Bubbles[i] = bubbleAroundText(s, t)
	}
	F.Texture = rowTexture(s)
	F.RowPanel = rowsCovered(F.Panels, s.H)
	F.RowText = rowsCovered(F.Text, s.H)
	F.RowBubble = rowsCovered(F.Bubbles, s.H)
	return F
}

// rowUniformity returns, per row, the largest per-channel standard deviation.
func rowUniformity(s *rgbStrip) []float64 {
	out := make([]float64, s.H)
	n := float64(s.W)
	for y := 0; y < s.H; y++ {
		row := s.Pix[y*s.W*3 : (y+1)*s.W*3]
		var sum, sq [3]float64
		for x := 0; x < s.W; x++ {
			for c := 0; c < 3; c++ {
				v := float64(row[x*3+c])
				sum[c] += v
				sq[c] += v * v
			}
		}
		best := 0.0
		for c := 0; c < 3; c++ {
			m := sum[c] / n
			v := sq[c]/n - m*m
			if v < 0 {
				v = 0
			}
			best = math.Max(best, math.Sqrt(v))
		}
		out[y] = best
	}
	return out
}

func findGutters(uni []float64) []rowSpan {
	var out []rowSpan
	start := -1
	for y := 0; y <= len(uni); y++ {
		flat := y < len(uni) && uni[y] < gutterStdThreshold
		if flat && start < 0 {
			start = y
		}
		if !flat && start >= 0 {
			if y-start >= gutterMinRows {
				out = append(out, rowSpan{start, y})
			}
			start = -1
		}
	}
	return out
}

// medianColour of all pixels in rows [a,b).
func medianColour(s *rgbStrip, a, b int) [3]uint8 {
	var hist [3][256]int
	for y := a; y < b; y++ {
		row := s.Pix[y*s.W*3 : (y+1)*s.W*3]
		for x := 0; x < s.W; x++ {
			hist[0][row[x*3]]++
			hist[1][row[x*3+1]]++
			hist[2][row[x*3+2]]++
		}
	}
	total := (b - a) * s.W
	var out [3]uint8
	for c := 0; c < 3; c++ {
		acc := 0
		for v := 0; v < 256; v++ {
			acc += hist[c][v]
			if acc*2 >= total {
				out[c] = uint8(v)
				break
			}
		}
	}
	return out
}

// backgroundPerRow assigns every row the colour of the nearest gutter.
func backgroundPerRow(s *rgbStrip, gutters []rowSpan) [][3]uint8 {
	bg := make([][3]uint8, s.H)
	if len(gutters) == 0 {
		// no gutters: use the left margin as background estimate
		edge := &rgbStrip{W: 10, H: s.H, Pix: make([]uint8, 10*s.H*3)}
		w := 10
		if s.W < w {
			w = s.W
		}
		edge.W = w
		edge.Pix = edge.Pix[:w*s.H*3]
		for y := 0; y < s.H; y++ {
			copy(edge.Pix[y*w*3:(y+1)*w*3], s.Pix[y*s.W*3:y*s.W*3+w*3])
		}
		c := medianColour(edge, 0, s.H)
		for y := range bg {
			bg[y] = c
		}
		return bg
	}
	centers := make([]int, len(gutters))
	cols := make([][3]uint8, len(gutters))
	for i, g := range gutters {
		centers[i] = (g.Start + g.End) / 2
		cols[i] = medianColour(s, g.Start, g.End)
	}
	j := 0
	for y := 0; y < s.H; y++ {
		for j+1 < len(centers) && abs(centers[j+1]-y) <= abs(centers[j]-y) {
			j++
		}
		bg[y] = cols[j]
	}
	return bg
}

func foregroundMask(s *rgbStrip, bg [][3]uint8) []uint8 {
	m := make([]uint8, s.W*s.H)
	for y := 0; y < s.H; y++ {
		row := s.Pix[y*s.W*3 : (y+1)*s.W*3]
		b := bg[y]
		for x := 0; x < s.W; x++ {
			d := max3(abs(int(row[x*3])-int(b[0])), abs(int(row[x*3+1])-int(b[1])), abs(int(row[x*3+2])-int(b[2])))
			if d > fgTolerance {
				m[y*s.W+x] = 1
			}
		}
	}
	return m
}

// --- binary morphology (separable, O(N)) ---

func dilate1D(in, out []uint8, n, stride, radius int) {
	// out[i] = any(in[i-radius..i+radius])
	cnt := 0
	for i := -radius; i <= radius && i < n; i++ {
		if i >= 0 && in[i*stride] != 0 {
			cnt++
		}
	}
	for i := 0; i < n; i++ {
		if cnt > 0 {
			out[i*stride] = 1
		} else {
			out[i*stride] = 0
		}
		if lo := i - radius; lo >= 0 && in[lo*stride] != 0 {
			cnt--
		}
		if hi := i + radius + 1; hi < n && in[hi*stride] != 0 {
			cnt++
		}
	}
}

func dilateMask(m []uint8, w, h, rx, ry int) []uint8 {
	tmp := make([]uint8, len(m))
	out := make([]uint8, len(m))
	for y := 0; y < h; y++ {
		dilate1D(m[y*w:], tmp[y*w:], w, 1, rx)
	}
	for x := 0; x < w; x++ {
		dilate1D(tmp[x:], out[x:], h, w, ry)
	}
	return out
}

func invertMask(m []uint8) []uint8 {
	out := make([]uint8, len(m))
	for i, v := range m {
		if v == 0 {
			out[i] = 1
		}
	}
	return out
}

func closeMask(m []uint8, w, h, r int) []uint8 {
	d := dilateMask(m, w, h, r, r)
	// erosion = complement of dilation of complement; clamp at the border (do not erode from outside)
	inv := invertMask(d)
	e := dilateMask(inv, w, h, r, r)
	out := invertMask(e)
	for y := 0; y < h; y++ {
		for x := 0; x < w; x++ {
			if (x < r || y < r || x >= w-r || y >= h-r) && d[y*w+x] != 0 {
				out[y*w+x] = 1
			}
		}
	}
	return out
}

// --- connected components (8-connectivity, union-find) ---

type component struct {
	box
	Area int
}

type unionFind struct{ parent []int32 }

func (u *unionFind) find(x int32) int32 {
	for u.parent[x] != x {
		u.parent[x] = u.parent[u.parent[x]]
		x = u.parent[x]
	}
	return x
}

func (u *unionFind) union(a, b int32) {
	ra, rb := u.find(a), u.find(b)
	if ra == rb {
		return
	}
	if ra < rb {
		u.parent[rb] = ra
	} else {
		u.parent[ra] = rb
	}
}

// labelComponents returns per-pixel labels (0 = background) and component stats indexed by label-1.
func labelComponents(m []uint8, w, h int) ([]int32, []component) {
	labels := make([]int32, w*h)
	uf := &unionFind{parent: []int32{0}}
	next := int32(1)
	for y := 0; y < h; y++ {
		for x := 0; x < w; x++ {
			i := y*w + x
			if m[i] == 0 {
				continue
			}
			var lab int32
			if y > 0 {
				for dx := -1; dx <= 1; dx++ {
					nx := x + dx
					if nx < 0 || nx >= w {
						continue
					}
					if l := labels[i-w+dx]; l != 0 {
						if lab == 0 {
							lab = l
						} else {
							uf.union(lab, l)
						}
					}
				}
			}
			if x > 0 {
				if l := labels[i-1]; l != 0 {
					if lab == 0 {
						lab = l
					} else {
						uf.union(lab, l)
					}
				}
			}
			if lab == 0 {
				lab = next
				next++
				uf.parent = append(uf.parent, lab)
			}
			labels[i] = lab
		}
	}
	// compress to dense labels
	remap := make([]int32, next)
	var comps []component
	for l := int32(1); l < next; l++ {
		r := uf.find(l)
		if remap[r] == 0 {
			comps = append(comps, component{box: box{w, h, 0, 0}})
			remap[r] = int32(len(comps))
		}
		remap[l] = remap[r]
	}
	for y := 0; y < h; y++ {
		for x := 0; x < w; x++ {
			i := y*w + x
			if labels[i] == 0 {
				continue
			}
			l := remap[labels[i]]
			labels[i] = l
			c := &comps[l-1]
			c.Area++
			if x < c.X0 {
				c.X0 = x
			}
			if y < c.Y0 {
				c.Y0 = y
			}
			if x+1 > c.X1 {
				c.X1 = x + 1
			}
			if y+1 > c.Y1 {
				c.Y1 = y + 1
			}
		}
	}
	return labels, comps
}

func panelBoxes(fg []uint8, w, h int) []box {
	closed := closeMask(fg, w, h, panelCloseKernel/2)
	_, comps := labelComponents(closed, w, h)
	var out []box
	for _, c := range comps {
		if c.Area >= panelMinArea && c.X1-c.X0 >= 20 && c.Y1-c.Y0 >= 20 {
			out = append(out, c.box)
		}
	}
	return out
}

// boxBlurChannel computes a (2r+1)^2 box mean of one channel, clamped at the borders like cv2.blur.
func boxBlurChannel(s *rgbStrip, c, r int) []uint8 {
	w, h := s.W, s.H
	tmp := make([]int32, w*h)
	// horizontal
	for y := 0; y < h; y++ {
		row := s.Pix[y*w*3 : (y+1)*w*3]
		var acc int32
		for x := -r; x <= r; x++ {
			acc += int32(row[clampInt(x, 0, w-1)*3+c])
		}
		for x := 0; x < w; x++ {
			tmp[y*w+x] = acc
			acc -= int32(row[clampInt(x-r, 0, w-1)*3+c])
			acc += int32(row[clampInt(x+r+1, 0, w-1)*3+c])
		}
	}
	out := make([]uint8, w*h)
	n := int32((2*r + 1) * (2*r + 1))
	for x := 0; x < w; x++ {
		var acc int32
		for y := -r; y <= r; y++ {
			acc += tmp[clampInt(y, 0, h-1)*w+x]
		}
		for y := 0; y < h; y++ {
			out[y*w+x] = uint8((acc + n/2) / n)
			acc -= tmp[clampInt(y-r, 0, h-1)*w+x]
			acc += tmp[clampInt(y+r+1, 0, h-1)*w+x]
		}
	}
	return out
}

// textBoxes finds typeset text blocks: clusters of letter-sized high-contrast blobs on a flat background.
func textBoxes(s *rgbStrip) []box {
	w, h := s.W, s.H
	neg := make([]uint8, w*h)
	pos := make([]uint8, w*h)
	for c := 0; c < 3; c++ {
		blur := boxBlurChannel(s, c, textBlurRadius)
		for i := 0; i < w*h; i++ {
			d := int(s.Pix[i*3+c]) - int(blur[i])
			if d > textContrastThr {
				pos[i] = 1
			} else if -d > textContrastThr {
				neg[i] = 1
			}
		}
	}
	var out []box
	for _, binm := range [][]uint8{neg, pos} {
		_, comps := labelComponents(binm, w, h)
		letters := make([]uint8, w*h)
		for _, c := range comps {
			cw, ch := c.X1-c.X0, c.Y1-c.Y0
			if ch >= 5 && ch <= 34 && cw >= 2 && cw <= 44 && c.Area >= 8 && float64(c.Area) <= 0.9*float64(cw*ch) {
				for y := c.Y0; y < c.Y1; y++ {
					for x := c.X0; x < c.X1; x++ {
						letters[y*w+x] = 1
					}
				}
			}
		}
		blocks := dilateMask(letters, w, h, 8, 4)
		_, bcomps := labelComponents(blocks, w, h)
		for _, b := range bcomps {
			bw, bh := b.X1-b.X0, b.Y1-b.Y0
			if !(bw >= 20 && bh >= 10 && float64(bw) < 0.8*float64(w) && bh < 400) {
				continue
			}
			if tb, ok := qualifyTextBlock(s, binm, letters, b.box); ok {
				out = append(out, tb)
			}
		}
	}
	return out
}

func qualifyTextBlock(s *rgbStrip, binm, letters []uint8, b box) (box, bool) {
	const minGlyphs = 3
	w := s.W
	bw, bh := b.X1-b.X0, b.Y1-b.Y0
	// glyph components inside the block
	sub := make([]uint8, bw*bh)
	for y := 0; y < bh; y++ {
		copy(sub[y*bw:(y+1)*bw], letters[(b.Y0+y)*w+b.X0:(b.Y0+y)*w+b.X1])
	}
	_, glyphs := labelComponents(sub, bw, bh)
	// ink runs along x (touching glyphs merge into one component, so count alternations too)
	runs, prev := 0, false
	for x := b.X0; x < b.X1; x++ {
		ink := false
		for y := b.Y0; y < b.Y1 && !ink; y++ {
			ink = binm[y*w+x] != 0
		}
		if ink && !prev {
			runs++
		}
		prev = ink
	}
	lineLike := bh <= 40 && float64(bw) >= 2.5*float64(bh) && runs >= 3
	if len(glyphs) < minGlyphs && !lineLike {
		return box{}, false
	}
	if len(glyphs) >= 2 && !lineLike {
		var sum, sq float64
		for _, g := range glyphs {
			hh := float64(g.Y1 - g.Y0)
			sum += hh
			sq += hh * hh
		}
		n := float64(len(glyphs))
		mean := sum / n
		std := math.Sqrt(math.Max(0, sq/n-mean*mean))
		if std > 0.6*mean {
			return box{}, false
		}
	}
	// background inside the block (away from glyphs) must be flat
	grown := dilateMask(sub, bw, bh, 2, 2)
	var cnt int
	var sum, sq [3]float64
	for y := 0; y < bh; y++ {
		for x := 0; x < bw; x++ {
			if grown[y*bw+x] != 0 {
				continue
			}
			p := s.Pix[((b.Y0+y)*w+b.X0+x)*3:]
			for c := 0; c < 3; c++ {
				v := float64(p[c])
				sum[c] += v
				sq[c] += v * v
			}
			cnt++
		}
	}
	if cnt < 50 {
		return box{}, false
	}
	limit := 26.0
	if lineLike {
		limit = 40.0
	}
	for c := 0; c < 3; c++ {
		m := sum[c] / float64(cnt)
		if math.Sqrt(math.Max(0, sq[c]/float64(cnt)-m*m)) > limit {
			return box{}, false
		}
	}
	return b, true
}

// bubbleAroundText grows a text block to the flat-colour balloon enclosing it (floating-range flood fill).
func bubbleAroundText(s *rgbStrip, t box) box {
	const (
		tol      = 20
		maxH     = 600
		maxWFrac = 0.7
	)
	w := s.W
	wy0, wy1 := maxInt(0, t.Y0-maxH), minInt(s.H, t.Y1+maxH)
	wh := wy1 - wy0
	seedX := (t.X0 + t.X1) / 2
	seedY := maxInt(0, t.Y0-3) - wy0
	visited := make([]uint8, w*wh)
	stack := []int{seedY*w + seedX}
	visited[stack[0]] = 1
	bx0, bx1, by0, by1 := seedX, seedX+1, seedY, seedY+1
	px := func(i int) []uint8 { return s.Pix[((wy0+i/w)*w+i%w)*3:][:3] }
	for len(stack) > 0 {
		i := stack[len(stack)-1]
		stack = stack[:len(stack)-1]
		y, x := i/w, i%w
		if x < bx0 {
			bx0 = x
		}
		if x+1 > bx1 {
			bx1 = x + 1
		}
		if y < by0 {
			by0 = y
		}
		if y+1 > by1 {
			by1 = y + 1
		}
		p := px(i)
		for _, d := range [4][2]int{{1, 0}, {-1, 0}, {0, 1}, {0, -1}} {
			nx, ny := x+d[0], y+d[1]
			if nx < 0 || ny < 0 || nx >= w || ny >= wh {
				continue
			}
			j := ny*w + nx
			if visited[j] != 0 {
				continue
			}
			q := px(j)
			if abs(int(p[0])-int(q[0])) <= tol && abs(int(p[1])-int(q[1])) <= tol && abs(int(p[2])-int(q[2])) <= tol {
				visited[j] = 1
				stack = append(stack, j)
			}
		}
	}
	by0 += wy0
	by1 += wy0
	if float64(bx1-bx0) > maxWFrac*float64(w) || by1-by0 > maxH {
		// leaked into the background: just pad the text box a little
		return box{maxInt(0, t.X0-10), maxInt(0, t.Y0-10), minInt(w, t.X1+10), minInt(s.H, t.Y1+10)}
	}
	return box{minInt(bx0, t.X0), minInt(by0, t.Y0), maxInt(bx1, t.X1), maxInt(by1, t.Y1)}
}

// rowTexture is the mean absolute horizontal luma gradient per row, normalised so the mean over rows is 1.
func rowTexture(s *rgbStrip) []float32 {
	out := make([]float32, s.H)
	total := 0.0
	for y := 0; y < s.H; y++ {
		row := s.Pix[y*s.W*3 : (y+1)*s.W*3]
		acc := 0
		prev := luma(row[0], row[1], row[2])
		for x := 1; x < s.W; x++ {
			l := luma(row[x*3], row[x*3+1], row[x*3+2])
			acc += abs(l - prev)
			prev = l
		}
		if s.W > 1 {
			out[y] = float32(acc) / float32(s.W-1)
		}
		total += float64(out[y])
	}
	mean := float32(total/float64(s.H)) + 1e-6
	for y := range out {
		out[y] /= mean
	}
	return out
}

func rowsCovered(boxes []box, h int) []float32 {
	r := make([]float32, h)
	for _, b := range boxes {
		for y := maxInt(0, b.Y0); y < minInt(h, b.Y1); y++ {
			r[y] = 1
		}
	}
	return r
}

func luma(r, g, b uint8) int { return (299*int(r) + 587*int(g) + 114*int(b)) / 1000 }

func abs(v int) int {
	if v < 0 {
		return -v
	}
	return v
}

func max3(a, b, c int) int { return maxInt(a, maxInt(b, c)) }

func maxInt(a, b int) int {
	if a > b {
		return a
	}
	return b
}

func minInt(a, b int) int {
	if a < b {
		return a
	}
	return b
}

func clampInt(v, lo, hi int) int {
	if v < lo {
		return lo
	}
	if v > hi {
		return hi
	}
	return v
}

var _ = sort.Ints
