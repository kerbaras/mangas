package cmd

import (
	"fmt"
	"os"
	"path/filepath"
	"strconv"
	"strings"

	"github.com/kerbaras/mangas/pkg/data"
	"github.com/kerbaras/mangas/pkg/integrations"
	"github.com/kerbaras/mangas/pkg/services"
	"github.com/kerbaras/mangas/pkg/sources"
	"github.com/spf13/cobra"
)

var downloadCmd = &cobra.Command{
	Use:   "download [manga-name or manga-id]",
	Short: "Download manga chapters",
	Long:  "Download chapters of a manga from your library or by ID",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		mangaIdentifier := args[0]
		language, _ := cmd.Flags().GetString("language")
		chaptersFlag, _ := cmd.Flags().GetString("chapters")

		repo := data.NewDuckDBRepository()
		source := sources.NewMangaDex()

		homeDir, _ := os.UserHomeDir()
		downloadDir := filepath.Join(homeDir, "Downloads")
		epubDir := filepath.Join(homeDir, "Downloads")

		processor := integrations.NewEPUBProcessor(epubDir)
		downloader := services.NewDownloader(source, repo, processor, downloadDir)

		// Try to find manga by name in library first
		var mangaID string
		mangas, _ := repo.ListMangas()
		for _, m := range mangas {
			if strings.EqualFold(m.Name, mangaIdentifier) {
				mangaID = m.ID
				fmt.Printf("📚 Found '%s' in library\n", m.Name)
				break
			}
		}

		// If not found in library, use as ID directly
		if mangaID == "" {
			mangaID = mangaIdentifier
			fmt.Printf("🔍 Downloading manga ID: %s\n", mangaID)
		}

		// Parse chapter range if specified
		var startChapter, endChapter int
		if chaptersFlag != "" {
			parts := strings.Split(chaptersFlag, "-")
			if len(parts) == 2 {
				startChapter, _ = strconv.Atoi(parts[0])
				endChapter, _ = strconv.Atoi(parts[1])
				fmt.Printf("📥 Downloading chapters %d-%d (language: %s)\n", startChapter, endChapter, language)
			} else {
				fmt.Println("⚠️  Invalid chapter range format. Use --chapters 1-10")
			}
		} else {
			fmt.Printf("📥 Downloading all chapters (language: %s)\n", language)
		}

		// Listen for progress
		go func() {
			for progress := range downloader.GetProgressChannel() {
				if progress.ChapterNumber != "" && progress.Status != "complete" {
					if progress.TotalPages > 0 {
						fmt.Printf("  Chapter %s: %d/%d pages\n", progress.ChapterNumber, progress.CurrentPage, progress.TotalPages)
					} else {
						fmt.Printf("  Chapter %s: %s\n", progress.ChapterNumber, progress.Status)
					}
				}
			}
		}()

		if err := downloader.DownloadManga(mangaID, language); err != nil {
			cobra.CheckErr(fmt.Errorf("download failed: %w", err))
		}

		fmt.Println("\n✅ Download complete! Generating EPUB...")
		epubPath, err := downloader.ComposeEPUB(mangaID)
		if err != nil {
			cobra.CheckErr(fmt.Errorf("EPUB generation failed: %w", err))
		}

		fmt.Printf("📖 EPUB created: %s\n", epubPath)
	},
}

func init() {
	downloadCmd.Flags().StringP("language", "l", "en", "Language code (e.g., en, ja, es)")
	downloadCmd.Flags().StringP("chapters", "c", "", "Chapter range (e.g., 1-10)")
}
