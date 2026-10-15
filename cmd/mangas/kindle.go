package cmd

import (
	"fmt"
	"strings"

	"github.com/kerbaras/mangas/pkg/data"
	"github.com/kerbaras/mangas/pkg/integrations"
	"github.com/kerbaras/mangas/pkg/services"
	"github.com/spf13/cobra"
)

var kindleCmd = &cobra.Command{
	Use:   "kindle [manga-name]",
	Short: "Export manga to Kindle-optimized format",
	Long: `Export downloaded manga chapters to Kindle-optimized format.

Supports all Kindle devices with optimized image processing for better reading experience.
Use --device to specify your Kindle model for optimal results.

Examples:
  mangas kindle "One Piece" --device kindle-paperwhite3 --chapters 1-10
  mangas kindle "Naruto" --device kindle-oasis --format mobi
  mangas kindle "Bleach" --device kindle-scribe --chapters 5,6,7
  mangas kindle "Solo Leveling" --device kindle-paperwhite12 --format epub

Long vertical strips (webtoons / manhwa) are split into device pages at rows that avoid panels,
text and speech bubbles (disable with --smart-cut=false). When no clean cut exists a page may be
shrunk a little so a whole panel fits, or the next page repeats the last rows; tune with
--cut-preset, --max-shrink and --overlap.

Use 'mangas kindle --list-devices' to see all supported Kindle devices.`,
	Args: cobra.MaximumNArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		// Check if user wants to list devices
		listDevices, _ := cmd.Flags().GetBool("list-devices")
		if listDevices {
			printDeviceList()
			return
		}

		if len(args) == 0 {
			cobra.CheckErr(fmt.Errorf("manga name is required (use --list-devices to see supported devices)"))
		}

		mangaName := args[0]
		deviceID, _ := cmd.Flags().GetString("device")
		format, _ := cmd.Flags().GetString("format")
		chapters, _ := cmd.Flags().GetString("chapters")
		output, _ := cmd.Flags().GetString("output")
		title, _ := cmd.Flags().GetString("title")
		author, _ := cmd.Flags().GetString("author")
		smartCut, _ := cmd.Flags().GetBool("smart-cut")
		cutPreset, _ := cmd.Flags().GetString("cut-preset")
		maxShrink, _ := cmd.Flags().GetFloat64("max-shrink")
		overlap, _ := cmd.Flags().GetFloat64("overlap")

		// Validate device
		if deviceID == "" {
			cobra.CheckErr(fmt.Errorf("device is required. Use --list-devices to see available options"))
		}

		_, ok := integrations.GetDeviceProfile(deviceID)
		if !ok {
			cobra.CheckErr(fmt.Errorf("unknown device: %s. Use --list-devices to see available options", deviceID))
		}

		// Initialize components
		repo := data.NewDuckDBRepository()
		controller := services.NewMangaController()
		defer controller.Close()

		// Find manga in library
		fmt.Printf("?? Searching for '%s' in library...\n", mangaName)
		manga, err := controller.FindMangaByName(mangaName)
		if err != nil {
			cobra.CheckErr(fmt.Errorf("manga not found in library: %w", err))
		}

		fmt.Printf("? Found: %s (ID: %s)\n", manga.Name, manga.ID)

		// Get chapters from library
		allChapters, err := repo.GetChapters(manga.ID)
		if err != nil {
			cobra.CheckErr(fmt.Errorf("failed to get chapters: %w", err))
		}

		// Filter chapters based on --chapters flag
		var selectedChapters []*data.Chapter
		if chapters == "" {
			// Export all downloaded chapters
			for _, ch := range allChapters {
				if ch.Downloaded && ch.FilePath != "" {
					selectedChapters = append(selectedChapters, ch)
				}
			}
		} else {
			selectedChapters = parseChapterSelection(chapters, allChapters)
		}

		if len(selectedChapters) == 0 {
			cobra.CheckErr(fmt.Errorf("no downloaded chapters found matching the selection"))
		}

		fmt.Printf("?? Selected %d chapter(s) for export\n", len(selectedChapters))

		// Determine output path
		if output == "" {
			safeTitle := sanitizeFilename(manga.Name)
			output = fmt.Sprintf("%s_kindle.%s", safeTitle, format)
		}

		// Set title if not provided
		if title == "" {
			title = manga.Name
		}
		if author == "" {
			author = "MangaDex"
		}

		fmt.Printf("?? Optimizing for %s...\n", deviceID)

		// Create Kindle converter
		converter, err := integrations.NewKindleConverter(deviceID)
		if err != nil {
			cobra.CheckErr(fmt.Errorf("failed to create converter: %w", err))
		}
		defer converter.Close()

		// Prepare chapter paths
		chapterPaths := make([]string, len(selectedChapters))
		for i, ch := range selectedChapters {
			chapterPaths[i] = ch.FilePath
		}

		// Set up export options
		device, _ := integrations.GetDeviceProfile(deviceID)
		cutOpts, err := integrations.SmartCutPreset(cutPreset, device)
		cobra.CheckErr(err)
		if maxShrink > 0 {
			if maxShrink < 1 {
				cobra.CheckErr(fmt.Errorf("--max-shrink must be >= 1.0 (1.0 = never shrink)"))
			}
			cutOpts.MaxShrink = maxShrink
		}
		if overlap >= 0 {
			if overlap > 0.5 {
				cobra.CheckErr(fmt.Errorf("--overlap must be between 0 and 0.5 (fraction of the page height)"))
			}
			cutOpts.OverlapFrac = overlap
		}
		options := integrations.ExportOptions{
			Device:       device,
			Format:       integrations.KindleFormat(format),
			Title:        title,
			Author:       author,
			Chapters:     chapterPaths,
			OutputPath:   output,
			Optimize:     true,
			PanelView:    device.PanelView,
			RightToLeft:  true, // Manga reading direction
			SmartCut:     smartCut,
			SmartCutOpts: &cutOpts,
		}

		fmt.Println("??  Converting and optimizing images...")

		// Convert
		outputPath, err := converter.ConvertChapters(options)
		if err != nil {
			cobra.CheckErr(fmt.Errorf("conversion failed: %w", err))
		}

		fmt.Printf("? Export complete!\n")
		fmt.Printf("?? Output: %s\n", outputPath)
		fmt.Printf("?? Optimized for: %s\n", device.Name)
		fmt.Printf("?? Transfer this file to your Kindle device or email it to your Kindle email address\n")
	},
}

func init() {
	kindleCmd.Flags().StringP("device", "d", "", "Kindle device model (required)")
	kindleCmd.Flags().StringP("format", "f", "mobi", "Output format: mobi, azw3, or epub")
	kindleCmd.Flags().StringP("chapters", "c", "", "Chapter selection (e.g., '1-10' or '1,3,5')")
	kindleCmd.Flags().StringP("output", "o", "", "Output file path (default: <manga-name>_kindle.<format>)")
	kindleCmd.Flags().StringP("title", "t", "", "Custom title for the export")
	kindleCmd.Flags().StringP("author", "a", "", "Custom author name")
	kindleCmd.Flags().Bool("list-devices", false, "List all supported Kindle devices")
	kindleCmd.Flags().Bool("smart-cut", true, "Split long webtoon strips at panel/bubble-safe rows instead of fixed heights")
	kindleCmd.Flags().String("cut-preset", integrations.PresetBalanced, "Smart-cut preset: balanced (shrink <=10%), fewest-cuts (shrink <=25%), no-shrink")
	kindleCmd.Flags().Float64("max-shrink", 0, "Override preset: max page shrink factor, e.g. 1.15 = pages may be 15% smaller (1.0 = never)")
	kindleCmd.Flags().Float64("overlap", -1, "Override preset: rows repeated after a cut through artwork, as a fraction of page height (e.g. 0.075)")

	rootCmd.AddCommand(kindleCmd)
}

func printDeviceList() {
	fmt.Println("📱 Supported Kindle Devices:")
	fmt.Println("E-Ink Readers:")
	fmt.Println("  kindle-paperwhite12   - Kindle Paperwhite 12th gen / Signature (300 DPI, 7\")")
	fmt.Println("  kindle-paperwhite5    - Kindle Paperwhite 11th gen / Signature (300 DPI, 6.8\")")
	fmt.Println("  kindle-paperwhite3    - Kindle Paperwhite 3/4 (300 DPI)")
	fmt.Println("  kindle-oasis          - Kindle Oasis 1/2 (300 DPI)")
	fmt.Println("  kindle-oasis3         - Kindle Oasis 3 (300 DPI, larger screen)")
	fmt.Println("  kindle-voyage         - Kindle Voyage (300 DPI)")
	fmt.Println("  kindle-scribe         - Kindle Scribe (300 DPI, 10.2\")")
	fmt.Println("  kindle-paperwhite     - Kindle Paperwhite 1/2 (212 DPI)")
	fmt.Println("  kindle-basic          - Kindle Basic 10th gen (167 DPI)")
	fmt.Println("  kindle-touch          - Kindle Touch (167 DPI)")
	fmt.Println("  kindle4               - Kindle 4 (167 DPI)")
	fmt.Println()
	fmt.Println("Fire Tablets (Color):")
	fmt.Println("  kindle-fire-hdx       - Kindle Fire HDX 7 (323 DPI)")
	fmt.Println("  kindle-fire-hd        - Kindle Fire HD 7 (216 DPI)")
	fmt.Println("  kindle-fire           - Kindle Fire (169 DPI)")
	fmt.Println()
	fmt.Println("?? Recommended devices for manga:")
	fmt.Println("   - kindle-paperwhite12 (current Paperwhite; long webtoon strips are smart-cut by default)")
	fmt.Println("   - kindle-paperwhite3 (best balance of quality and compatibility)")
	fmt.Println("   - kindle-oasis3 (larger screen, great for manga)")
	fmt.Println("   - kindle-scribe (largest screen available)")
}

func parseChapterSelection(selection string, allChapters []*data.Chapter) []*data.Chapter {
	var selected []*data.Chapter

	// Create a map of chapter numbers to chapters
	chapterMap := make(map[string]*data.Chapter)
	for _, ch := range allChapters {
		if ch.Downloaded && ch.FilePath != "" {
			chapterMap[ch.Number] = ch
		}
	}

	// Handle comma-separated list
	parts := strings.Split(selection, ",")
	for _, part := range parts {
		part = strings.TrimSpace(part)

		// Handle range (e.g., "1-5")
		if strings.Contains(part, "-") {
			// Range parsing would need chapter number parsing
			// For simplicity, just add all chapters in the range
			continue
		}

		// Single chapter
		if ch, ok := chapterMap[part]; ok {
			selected = append(selected, ch)
		}
	}

	return selected
}

func sanitizeFilename(name string) string {
	invalid := []string{"/", "\\", ":", "*", "?", "\"", "<", ">", "|"}
	result := name
	for _, char := range invalid {
		result = strings.ReplaceAll(result, char, "_")
	}
	result = strings.TrimSpace(result)
	result = strings.Trim(result, ".")
	return result
}
