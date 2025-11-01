package components

import (
	"strings"
	"testing"

	"github.com/kerbaras/mangas/pkg/services"
)

func TestNewProgressTracker(t *testing.T) {
	tracker := NewProgressTracker(80)

	if tracker == nil {
		t.Fatal("Expected tracker to be created")
	}

	if tracker.width != 80 {
		t.Errorf("Expected width 80, got %d", tracker.width)
	}

	if len(tracker.downloads) != 0 {
		t.Errorf("Expected 0 downloads, got %d", len(tracker.downloads))
	}
}

func TestUpdate(t *testing.T) {
	tracker := NewProgressTracker(80)

	progress := services.DownloadProgress{
		MangaID:       "manga-1",
		ChapterID:     "ch-1",
		ChapterNumber: "1",
		Status:        "downloading",
		TotalPages:    10,
		CurrentPage:   5,
	}

	tracker.Update(progress)

	if !tracker.HasActive() {
		t.Error("Expected tracker to have active downloads")
	}

	if len(tracker.downloads) != 1 {
		t.Errorf("Expected 1 download, got %d", len(tracker.downloads))
	}
}

func TestUpdateRemovesCompleted(t *testing.T) {
	tracker := NewProgressTracker(80)

	progress := services.DownloadProgress{
		MangaID:       "manga-1",
		ChapterID:     "ch-1",
		ChapterNumber: "1",
		Status:        "downloading",
	}

	tracker.Update(progress)

	if len(tracker.downloads) != 1 {
		t.Errorf("Expected 1 download, got %d", len(tracker.downloads))
	}

	// Mark as complete
	progress.Status = "complete"
	tracker.Update(progress)

	if len(tracker.downloads) != 0 {
		t.Errorf("Expected completed download to be removed, got %d", len(tracker.downloads))
	}
}

func TestClear(t *testing.T) {
	tracker := NewProgressTracker(80)

	// Add some downloads
	for i := 1; i <= 3; i++ {
		progress := services.DownloadProgress{
			MangaID:   "manga-1",
			ChapterID: string(rune('a' + i)),
			Status:    "downloading",
		}
		tracker.Update(progress)
	}

	if len(tracker.downloads) != 3 {
		t.Errorf("Expected 3 downloads, got %d", len(tracker.downloads))
	}

	tracker.Clear()

	if len(tracker.downloads) != 0 {
		t.Errorf("Expected 0 downloads after clear, got %d", len(tracker.downloads))
	}
}

func TestHasActive(t *testing.T) {
	tracker := NewProgressTracker(80)

	if tracker.HasActive() {
		t.Error("Expected no active downloads initially")
	}

	progress := services.DownloadProgress{
		MangaID:   "manga-1",
		ChapterID: "ch-1",
		Status:    "downloading",
	}

	tracker.Update(progress)

	if !tracker.HasActive() {
		t.Error("Expected active downloads after update")
	}

	tracker.Clear()

	if tracker.HasActive() {
		t.Error("Expected no active downloads after clear")
	}
}

func TestViewEmpty(t *testing.T) {
	tracker := NewProgressTracker(80)

	view := tracker.View()

	if view != "" {
		t.Errorf("Expected empty view, got: %s", view)
	}
}

func TestViewWithProgress(t *testing.T) {
	tracker := NewProgressTracker(80)

	progress := services.DownloadProgress{
		MangaID:       "manga-1",
		ChapterID:     "ch-1",
		ChapterNumber: "5",
		Status:        "downloading",
		TotalPages:    20,
		CurrentPage:   10,
	}

	tracker.Update(progress)

	view := tracker.View()

	if !strings.Contains(view, "Active Downloads") {
		t.Error("Expected 'Active Downloads' header")
	}

	if !strings.Contains(view, "Chapter 5") {
		t.Error("Expected chapter number in view")
	}

	if !strings.Contains(view, "downloading") {
		t.Error("Expected status in view")
	}

	if !strings.Contains(view, "10/20") {
		t.Error("Expected page progress in view")
	}
}

func TestRenderProgressBar(t *testing.T) {
	bar := renderProgressBar(50, 100, 20)

	if len(bar) < 20 {
		t.Errorf("Expected progress bar of at least 20 chars, got %d", len(bar))
	}

	// Should contain filled and unfilled characters
	if !strings.Contains(bar, "█") && !strings.Contains(bar, "░") {
		t.Error("Expected progress bar to contain progress characters")
	}
}

func TestRenderProgressBarZeroTotal(t *testing.T) {
	bar := renderProgressBar(0, 0, 20)

	if bar != "" {
		t.Errorf("Expected empty string for zero total, got: %s", bar)
	}
}

func TestRenderProgressBarFull(t *testing.T) {
	bar := renderProgressBar(100, 100, 20)

	// Should be all filled
	expectedFilled := 20
	actualFilled := strings.Count(bar, "█")

	if actualFilled < expectedFilled {
		t.Errorf("Expected %d filled chars, got %d", expectedFilled, actualFilled)
	}
}

func TestSimpleProgress(t *testing.T) {
	bar := SimpleProgress(25, 100, 40)

	if bar == "" {
		t.Error("Expected non-empty progress bar")
	}

	// Should have some filled and some empty
	filled := strings.Count(bar, "█")
	empty := strings.Count(bar, "░")

	if filled == 0 {
		t.Error("Expected some filled characters")
	}

	if empty == 0 {
		t.Error("Expected some empty characters")
	}

	// Approximate check: 25% of 40 = 10 filled
	if filled < 8 || filled > 12 {
		t.Errorf("Expected approximately 10 filled chars, got %d", filled)
	}
}

func TestUpdateMultipleChapters(t *testing.T) {
	tracker := NewProgressTracker(80)

	// Add multiple chapters
	for i := 1; i <= 3; i++ {
		progress := services.DownloadProgress{
			MangaID:       "manga-1",
			ChapterID:     string(rune('a' + i - 1)),
			ChapterNumber: string(rune('0' + i)),
			Status:        "downloading",
		}
		tracker.Update(progress)
	}

	if len(tracker.downloads) != 3 {
		t.Errorf("Expected 3 downloads, got %d", len(tracker.downloads))
	}

	view := tracker.View()

	// Should contain all chapters
	for i := 1; i <= 3; i++ {
		expected := "Chapter " + string(rune('0'+i))
		if !strings.Contains(view, expected) {
			t.Errorf("Expected '%s' in view", expected)
		}
	}
}

func TestProgressWithError(t *testing.T) {
	tracker := NewProgressTracker(80)

	progress := services.DownloadProgress{
		MangaID:       "manga-1",
		ChapterID:     "ch-1",
		ChapterNumber: "1",
		Status:        "error",
		Error:         &testError{"download failed"},
	}

	tracker.Update(progress)

	view := tracker.View()

	if !strings.Contains(view, "Error:") {
		t.Error("Expected error message in view")
	}

	if !strings.Contains(view, "download failed") {
		t.Error("Expected error details in view")
	}
}

type testError struct {
	msg string
}

func (e *testError) Error() string {
	return e.msg
}

