package integrations

import (
	"archive/zip"
	"encoding/json"
	"image"
	"os"
	"path/filepath"
	"sort"
	"testing"
	"time"
)

// TestSmartCut_RealChapter runs the paginator over a real chapter when MANGAS_SMARTCUT_DIR points at a
// directory of strip images, and writes the chosen page spans (rows of the concatenated strip) to
// MANGAS_SMARTCUT_OUT as JSON so they can be scored against the Python reference pipeline.
func TestSmartCut_RealChapter(t *testing.T) {
	dir := os.Getenv("MANGAS_SMARTCUT_DIR")
	if dir == "" {
		t.Skip("MANGAS_SMARTCUT_DIR not set")
	}
	files, _ := filepath.Glob(filepath.Join(dir, "*"))
	sort.Strings(files)
	dev, _ := GetDeviceProfile("kindle-paperwhite12")
	p := NewStripPaginator(DefaultSmartCutOptions(dev))
	var spans [][2]int
	offset := 0 // global row of the top of the current unit
	carryH := 0
	started := time.Now()
	pagesRendered := 0
	for i, f := range files {
		data, err := os.ReadFile(f)
		if err != nil {
			t.Fatal(err)
		}
		img, _, err := decodeImage(data)
		if err != nil {
			t.Fatalf("%s: %v", f, err)
		}
		unit := stackImages(p.carry, img)
		p.carry = nil
		pages, src, rem := p.paginateUnit(unit, i == len(files)-1)
		pagesRendered += len(pages)
		for _, s := range src {
			spans = append(spans, [2]int{offset + s.Start, offset + s.End})
		}
		unitH := unit.Bounds().Dy()
		if rem != nil {
			carryH = rem.Bounds().Dy()
			p.carry = rem
		} else {
			carryH = 0
		}
		offset += unitH - carryH
	}
	t.Logf("%d pages in %s", pagesRendered, time.Since(started))
	if os.Getenv("MANGAS_SMARTCUT_TIME_FILTERS") != "" {
		// time the e-ink filters + JPEG encoding over a chapter's worth of pages
		proc := NewImageProcessor(dev.GetOptimizationSettings())
		q := NewStripPaginator(DefaultSmartCutOptions(dev))
		var all []*image.RGBA
		for i, f := range files {
			data, _ := os.ReadFile(f)
			img, _, _ := decodeImage(data)
			all = append(all, q.Feed(img, i == len(files)-1)...)
		}
		started = time.Now()
		var total int
		for _, pg := range all {
			out, err := proc.ProcessDecoded(pg)
			if err != nil {
				t.Fatal(err)
			}
			total += len(out)
		}
		t.Logf("filters+encode: %d pages in %s (%d KB)", len(all), time.Since(started), total/1024)
	}
	if out := os.Getenv("MANGAS_SMARTCUT_OUT"); out != "" {
		b, _ := json.Marshal(spans)
		if err := os.WriteFile(out, b, 0o644); err != nil {
			t.Fatal(err)
		}
	}
	if sample := os.Getenv("MANGAS_SMARTCUT_SAMPLES"); sample != "" {
		// re-run on the first two files and dump a few pages for eyeballing
		q := NewStripPaginator(DefaultSmartCutOptions(dev))
		var all []*image.RGBA
		for i, f := range files[:2] {
			data, _ := os.ReadFile(f)
			img, _, _ := decodeImage(data)
			all = append(all, q.Feed(img, i == 1)...)
		}
		for i, pg := range all {
			if i >= 12 {
				break
			}
			writePNG(t, filepath.Join(sample, "page_"+string(rune('a'+i))+".png"), pg)
		}
	}
}

// TestKindleConverter_RealChapterEndToEnd packs the strips of MANGAS_SMARTCUT_DIR into a chapter EPUB and
// runs the full Kindle export with smart cutting, checking that every strip became several device pages.
func TestKindleConverter_RealChapterEndToEnd(t *testing.T) {
	dir := os.Getenv("MANGAS_SMARTCUT_DIR")
	if dir == "" {
		t.Skip("MANGAS_SMARTCUT_DIR not set")
	}
	files, _ := filepath.Glob(filepath.Join(dir, "*"))
	sort.Strings(files)
	tmp := t.TempDir()
	chapter := filepath.Join(tmp, "chapter.epub")
	zf, err := os.Create(chapter)
	if err != nil {
		t.Fatal(err)
	}
	zw := zip.NewWriter(zf)
	for _, f := range files {
		data, _ := os.ReadFile(f)
		w, _ := zw.Create("OEBPS/images/" + filepath.Base(f))
		w.Write(data)
	}
	zw.Close()
	zf.Close()

	conv, err := NewKindleConverter("kindle-paperwhite12")
	if err != nil {
		t.Fatal(err)
	}
	defer conv.Close()
	started := time.Now()
	out, err := conv.ConvertChapters(ExportOptions{
		Device: KindleDevices["kindle-paperwhite12"], Format: "epub", Title: "smartcut e2e",
		Chapters: []string{chapter}, OutputPath: filepath.Join(tmp, "out.epub"), SmartCut: true,
	})
	if err != nil {
		t.Fatal(err)
	}
	r, err := zip.OpenReader(out)
	if err != nil {
		t.Fatal(err)
	}
	defer r.Close()
	pages := 0
	for _, f := range r.File {
		if filepath.Ext(f.Name) == ".jpg" {
			pages++
		}
	}
	t.Logf("%s: %d pages in %s", out, pages, time.Since(started))
	if pages < 5*len(files) {
		t.Fatalf("expected many device pages per strip, got %d pages for %d strips", pages, len(files))
	}
	if keep := os.Getenv("MANGAS_SMARTCUT_KEEP_EPUB"); keep != "" {
		data, _ := os.ReadFile(out)
		os.WriteFile(keep, data, 0o644)
	}
}
