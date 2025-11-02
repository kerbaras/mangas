package integrations

import (
	"bytes"
	"image"
	"image/color"
	"image/png"
	"testing"
)

func TestGetDeviceProfile(t *testing.T) {
	tests := []struct {
		name     string
		deviceID string
		wantOK   bool
	}{
		{"valid paperwhite", "kindle-paperwhite3", true},
		{"valid oasis", "kindle-oasis", true},
		{"invalid device", "invalid-device", false},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			device, ok := GetDeviceProfile(tt.deviceID)
			if ok != tt.wantOK {
				t.Errorf("GetDeviceProfile() ok = %v, want %v", ok, tt.wantOK)
			}
			if ok && device.Name == "" {
				t.Error("Device name should not be empty")
			}
		})
	}
}

func TestKindleDevice_GetOptimizationSettings(t *testing.T) {
	device := KindleDevice{
		Name:      "Test Kindle",
		Width:     1072,
		Height:    1448,
		DPI:       300,
		Grayscale: true,
	}

	settings := device.GetOptimizationSettings()

	if settings.MaxWidth != device.Width {
		t.Errorf("MaxWidth = %d, want %d", settings.MaxWidth, device.Width)
	}
	if settings.MaxHeight != device.Height {
		t.Errorf("MaxHeight = %d, want %d", settings.MaxHeight, device.Height)
	}
	if !settings.Grayscale {
		t.Error("Settings should be grayscale for e-ink device")
	}
	if !settings.Sharpen {
		t.Error("Sharpening should be enabled for e-ink device")
	}
}

func TestImageProcessor_CalculateDimensions(t *testing.T) {
	settings := ImageOptimizationSettings{
		MaxWidth:  800,
		MaxHeight: 1200,
	}
	processor := NewImageProcessor(settings)

	tests := []struct {
		name       string
		width      int
		height     int
		wantWidth  int
		wantHeight int
	}{
		{"no resize needed", 600, 800, 600, 800},
		{"resize width", 1000, 800, 800, 640},
		{"resize height", 800, 1500, 640, 1200},
		{"resize both", 1600, 2400, 800, 1200},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			gotWidth, gotHeight := processor.calculateDimensions(tt.width, tt.height)
			if gotWidth != tt.wantWidth || gotHeight != tt.wantHeight {
				t.Errorf("calculateDimensions() = (%d, %d), want (%d, %d)",
					gotWidth, gotHeight, tt.wantWidth, tt.wantHeight)
			}
		})
	}
}

func TestImageProcessor_ProcessImage(t *testing.T) {
	t.Run("process simple image", func(t *testing.T) {
		// Create a simple test image
		img := image.NewRGBA(image.Rect(0, 0, 100, 100))
		for y := 0; y < 100; y++ {
			for x := 0; x < 100; x++ {
				img.Set(x, y, color.RGBA{uint8(x), uint8(y), 128, 255})
			}
		}

		// Encode to PNG
		var buf bytes.Buffer
		if err := png.Encode(&buf, img); err != nil {
			t.Fatalf("Failed to encode test image: %v", err)
		}

		// Process
		settings := ImageOptimizationSettings{
			MaxWidth:  50,
			MaxHeight: 50,
			Quality:   85,
			Grayscale: false,
			Format:    "jpeg",
		}
		processor := NewImageProcessor(settings)

		result, err := processor.ProcessImageData(buf.Bytes())
		if err != nil {
			t.Fatalf("ProcessImageData() error = %v", err)
		}

		if len(result) == 0 {
			t.Error("Processed image should not be empty")
		}
	})

	t.Run("convert to grayscale", func(t *testing.T) {
		// Create colored image
		img := image.NewRGBA(image.Rect(0, 0, 50, 50))
		for y := 0; y < 50; y++ {
			for x := 0; x < 50; x++ {
				img.Set(x, y, color.RGBA{255, 0, 0, 255}) // Red
			}
		}

		var buf bytes.Buffer
		png.Encode(&buf, img)

		settings := ImageOptimizationSettings{
			MaxWidth:  50,
			MaxHeight: 50,
			Quality:   85,
			Grayscale: true,
			Format:    "jpeg",
		}
		processor := NewImageProcessor(settings)

		result, err := processor.ProcessImageData(buf.Bytes())
		if err != nil {
			t.Fatalf("ProcessImageData() error = %v", err)
		}

		if len(result) == 0 {
			t.Error("Processed image should not be empty")
		}
	})
}

func TestImageProcessor_ToGrayscale(t *testing.T) {
	// Create colored image
	img := image.NewRGBA(image.Rect(0, 0, 10, 10))
	for y := 0; y < 10; y++ {
		for x := 0; x < 10; x++ {
			img.Set(x, y, color.RGBA{255, 0, 0, 255})
		}
	}

	processor := &ImageProcessor{}
	gray := processor.toGrayscale(img)

	// Verify it's grayscale
	_, ok := gray.(*image.Gray)
	if !ok {
		t.Error("Result should be a grayscale image")
	}
}

func TestImageProcessor_AdjustContrast(t *testing.T) {
	// Create test image
	img := image.NewRGBA(image.Rect(0, 0, 10, 10))
	for y := 0; y < 10; y++ {
		for x := 0; x < 10; x++ {
			img.Set(x, y, color.RGBA{128, 128, 128, 255})
		}
	}

	processor := &ImageProcessor{}
	adjusted := processor.adjustContrast(img, 1.5)

	if adjusted == nil {
		t.Error("Adjusted image should not be nil")
	}

	// Verify dimensions maintained
	if adjusted.Bounds() != img.Bounds() {
		t.Error("Image dimensions should be maintained")
	}
}

func TestImageProcessor_Clamp(t *testing.T) {
	tests := []struct {
		input int32
		want  uint8
	}{
		{-10, 0},
		{0, 0},
		{128, 128},
		{255, 255},
		{300, 255},
	}

	for _, tt := range tests {
		got := clamp(tt.input)
		if got != tt.want {
			t.Errorf("clamp(%d) = %d, want %d", tt.input, got, tt.want)
		}
	}
}

func TestKindleConverter_New(t *testing.T) {
	t.Run("valid device", func(t *testing.T) {
		converter, err := NewKindleConverter("kindle-paperwhite3")
		if err != nil {
			t.Fatalf("NewKindleConverter() error = %v", err)
		}
		defer converter.Close()

		if converter.device.Name == "" {
			t.Error("Device name should be set")
		}
		if converter.processor == nil {
			t.Error("Processor should be initialized")
		}
	})

	t.Run("invalid device", func(t *testing.T) {
		_, err := NewKindleConverter("invalid-device")
		if err == nil {
			t.Error("NewKindleConverter() should fail with invalid device")
		}
	})
}

func TestListDevices(t *testing.T) {
	devices := ListDevices()
	
	if len(devices) == 0 {
		t.Error("ListDevices() should return at least one device")
	}

	// Check format
	for _, device := range devices {
		if !contains(device, ":") {
			t.Errorf("Device entry should contain ':' separator: %s", device)
		}
	}
}

func contains(s, substr string) bool {
	return len(s) > 0 && len(substr) > 0 && (s == substr || len(s) > len(substr) && (s[:len(substr)] == substr || s[len(s)-len(substr):] == substr || containsMiddle(s, substr)))
}

func containsMiddle(s, substr string) bool {
	for i := 0; i <= len(s)-len(substr); i++ {
		if s[i:i+len(substr)] == substr {
			return true
		}
	}
	return false
}

func TestDeviceProfiles_Coverage(t *testing.T) {
	// Verify all important Kindle models are present
	required := []string{
		"kindle-paperwhite3",
		"kindle-oasis",
		"kindle-voyage",
		"kindle-scribe",
	}

	for _, deviceID := range required {
		t.Run(deviceID, func(t *testing.T) {
			device, ok := GetDeviceProfile(deviceID)
			if !ok {
				t.Errorf("Device %s should be available", deviceID)
			}
			if device.Width == 0 || device.Height == 0 {
				t.Error("Device dimensions should be set")
			}
			if device.DPI == 0 {
				t.Error("Device DPI should be set")
			}
		})
	}
}

func BenchmarkImageProcessor_ProcessImage(b *testing.B) {
	// Create test image
	img := image.NewRGBA(image.Rect(0, 0, 800, 1200))
	for y := 0; y < 1200; y++ {
		for x := 0; x < 800; x++ {
			img.Set(x, y, color.RGBA{uint8(x % 256), uint8(y % 256), 128, 255})
		}
	}

	var buf bytes.Buffer
	png.Encode(&buf, img)
	imageData := buf.Bytes()

	settings := ImageOptimizationSettings{
		MaxWidth:  758,
		MaxHeight: 1024,
		Quality:   85,
		Grayscale: true,
		Sharpen:   true,
		Format:    "jpeg",
	}
	processor := NewImageProcessor(settings)

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		processor.ProcessImageData(imageData)
	}
}
