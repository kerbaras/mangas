package integrations

// KindleDevice represents different Kindle device models with their specifications
type KindleDevice struct {
	Name        string
	Model       string
	Width       int  // Screen width in pixels
	Height      int  // Screen height in pixels
	DPI         int  // Dots per inch
	Grayscale   bool // Whether device supports only grayscale
	PanelView   bool // Supports panel view mode
	Orientation string // "portrait" or "landscape" or "both"
}

// Predefined Kindle device profiles based on actual hardware
var KindleDevices = map[string]KindleDevice{
	"kindle1": {
		Name:        "Kindle 1",
		Model:       "K1",
		Width:       600,
		Height:      800,
		DPI:         167,
		Grayscale:   true,
		PanelView:   false,
		Orientation: "portrait",
	},
	"kindle2": {
		Name:        "Kindle 2",
		Model:       "K2",
		Width:       600,
		Height:      800,
		DPI:         167,
		Grayscale:   true,
		PanelView:   false,
		Orientation: "portrait",
	},
	"kindle-dx": {
		Name:        "Kindle DX",
		Model:       "KDX",
		Width:       824,
		Height:      1200,
		DPI:         150,
		Grayscale:   true,
		PanelView:   false,
		Orientation: "portrait",
	},
	"kindle3": {
		Name:        "Kindle Keyboard",
		Model:       "K3",
		Width:       600,
		Height:      800,
		DPI:         167,
		Grayscale:   true,
		PanelView:   false,
		Orientation: "portrait",
	},
	"kindle4": {
		Name:        "Kindle 4",
		Model:       "K4",
		Width:       600,
		Height:      800,
		DPI:         167,
		Grayscale:   true,
		PanelView:   false,
		Orientation: "portrait",
	},
	"kindle-touch": {
		Name:        "Kindle Touch",
		Model:       "KT",
		Width:       600,
		Height:      800,
		DPI:         167,
		Grayscale:   true,
		PanelView:   false,
		Orientation: "portrait",
	},
	"kindle-paperwhite": {
		Name:        "Kindle Paperwhite 1/2",
		Model:       "KPW",
		Width:       758,
		Height:      1024,
		DPI:         212,
		Grayscale:   true,
		PanelView:   true,
		Orientation: "portrait",
	},
	"kindle-paperwhite3": {
		Name:        "Kindle Paperwhite 3/4",
		Model:       "KPW3",
		Width:       1072,
		Height:      1448,
		DPI:         300,
		Grayscale:   true,
		PanelView:   true,
		Orientation: "portrait",
	},
	"kindle-voyage": {
		Name:        "Kindle Voyage",
		Model:       "KV",
		Width:       1072,
		Height:      1448,
		DPI:         300,
		Grayscale:   true,
		PanelView:   true,
		Orientation: "portrait",
	},
	"kindle-oasis": {
		Name:        "Kindle Oasis 1/2",
		Model:       "KO",
		Width:       1072,
		Height:      1448,
		DPI:         300,
		Grayscale:   true,
		PanelView:   true,
		Orientation: "both",
	},
	"kindle-oasis3": {
		Name:        "Kindle Oasis 3",
		Model:       "KO3",
		Width:       1264,
		Height:      1680,
		DPI:         300,
		Grayscale:   true,
		PanelView:   true,
		Orientation: "both",
	},
	"kindle-basic": {
		Name:        "Kindle Basic (10th gen)",
		Model:       "KB",
		Width:       758,
		Height:      1024,
		DPI:         167,
		Grayscale:   true,
		PanelView:   false,
		Orientation: "portrait",
	},
	"kindle-scribe": {
		Name:        "Kindle Scribe",
		Model:       "KS",
		Width:       1860,
		Height:      2480,
		DPI:         300,
		Grayscale:   true,
		PanelView:   true,
		Orientation: "both",
	},
	// Fire tablets (color screens)
	"kindle-fire": {
		Name:        "Kindle Fire",
		Model:       "KF",
		Width:       600,
		Height:      1024,
		DPI:         169,
		Grayscale:   false,
		PanelView:   true,
		Orientation: "both",
	},
	"kindle-fire-hd": {
		Name:        "Kindle Fire HD 7",
		Model:       "KFHD7",
		Width:       800,
		Height:      1280,
		DPI:         216,
		Grayscale:   false,
		PanelView:   true,
		Orientation: "both",
	},
	"kindle-fire-hdx": {
		Name:        "Kindle Fire HDX 7",
		Model:       "KFHDX7",
		Width:       1200,
		Height:      1920,
		DPI:         323,
		Grayscale:   false,
		PanelView:   true,
		Orientation: "both",
	},
}

// GetDeviceProfile returns the device profile for a given device ID
func GetDeviceProfile(deviceID string) (KindleDevice, bool) {
	device, ok := KindleDevices[deviceID]
	return device, ok
}

// ListDevices returns a list of all available device IDs and names
func ListDevices() []string {
	devices := make([]string, 0, len(KindleDevices))
	for id, device := range KindleDevices {
		devices = append(devices, id+": "+device.Name)
	}
	return devices
}

// ImageOptimizationSettings defines how images should be processed for Kindle
type ImageOptimizationSettings struct {
	MaxWidth      int     // Maximum image width
	MaxHeight     int     // Maximum image height
	Quality       int     // JPEG quality (1-100)
	Grayscale     bool    // Convert to grayscale
	Sharpen       bool    // Apply sharpening for e-ink
	Contrast      float64 // Contrast adjustment (1.0 = no change)
	Gamma         float64 // Gamma correction for e-ink
	Format        string  // Output format: "jpeg" or "png"
	StripMetadata bool    // Remove EXIF data to reduce size
}

// GetOptimizationSettings returns recommended settings for a device
func (d KindleDevice) GetOptimizationSettings() ImageOptimizationSettings {
	settings := ImageOptimizationSettings{
		MaxWidth:      d.Width,
		MaxHeight:     d.Height,
		Quality:       85,
		Grayscale:     d.Grayscale,
		Sharpen:       d.Grayscale, // Only sharpen for e-ink displays
		Contrast:      1.1,          // Slightly boost contrast for e-ink
		Gamma:         1.0,
		Format:        "jpeg",
		StripMetadata: true,
	}

	// High DPI devices can use better quality
	if d.DPI >= 300 {
		settings.Quality = 90
	}

	// E-ink devices benefit from gamma adjustment
	if d.Grayscale {
		settings.Gamma = 0.9 // Slightly darker for better e-ink rendering
	}

	return settings
}

// KindleFormat represents the output format for Kindle
type KindleFormat string

const (
	FormatMOBI KindleFormat = "mobi" // Legacy MOBI format
	FormatAZW3 KindleFormat = "azw3" // Kindle Format 8 (KF8)
	FormatKFX  KindleFormat = "kfx"  // Latest Kindle format
)

// ExportOptions defines options for exporting to Kindle format
type ExportOptions struct {
	Device       KindleDevice
	Format       KindleFormat
	Title        string
	Author       string
	Chapters     []string // Chapter IDs or file paths
	OutputPath   string
	Optimize     bool // Apply image optimization
	PanelView    bool // Enable panel view mode
	RightToLeft  bool // For manga reading direction
	CoverImage   string // Path to custom cover image
}
