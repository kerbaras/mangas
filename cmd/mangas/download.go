package cmd

import (
	"fmt"
	"os"
	"path/filepath"
	"strconv"
	"strings"

	"github.com/kerbaras/mangas/pkg/data"
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
		downloadDir := filepath.Join(homeDir, ".mangas", "downloads")

		downloader := services.NewDownloader(source, repo, downloadDir)
		defer downloader.Close()

		// Try to find manga by name in library first
		var manga *data.Manga
		mangas, _ := repo.ListMangas()
		for _, m := range mangas {
			if strings.EqualFold(m.Name, mangaIdentifier) {
				manga = m
				fmt.Printf("📚 Found '%s' in library\n", m.Name)
				break
			}
		}

		// If not found in library, fetch from source
		if manga == nil {
			var err error
			manga, err = source.GetManga(mangaIdentifier)
			if err != nil {
				cobra.CheckErr(fmt.Errorf("manga not found: %w", err))
			}
			fmt.Printf("🔍 Found manga: %s (ID: %s)\n", manga.Name, manga.ID)
		}

		// Get chapters from source
		chapters, err := source.GetChapters(manga)
		if err != nil {
			cobra.CheckErr(fmt.Errorf("failed to get chapters: %w", err))
		}

		// Filter by language
		var filteredChapters []*data.Chapter
		for _, ch := range chapters {
			if ch.Language == language {
				filteredChapters = append(filteredChapters, ch)
			}
		}

		// Filter by chapter range if specified
		var startChapter, endChapter int
		if chaptersFlag != "" {
			parts := strings.Split(chaptersFlag, "-")
			if len(parts) == 2 {
				startChapter, _ = strconv.Atoi(parts[0])
				endChapter, _ = strconv.Atoi(parts[1])
				fmt.Printf("📥 Downloading chapters %d-%d (language: %s)\n", startChapter, endChapter, language)
				
				var rangeChapters []*data.Chapter
				for _, ch := range filteredChapters {
					chNum, _ := strconv.ParseFloat(ch.Number, 64)
					if chNum >= float64(startChapter) && chNum <= float64(endChapter) {
						rangeChapters = append(rangeChapters, ch)
					}
				}
				filteredChapters = rangeChapters
			} else {
				fmt.Println("⚠️  Invalid chapter range format. Use --chapters 1-10")
			}
		} else {
			fmt.Printf("📥 Downloading %d chapters (language: %s)\n", len(filteredChapters), language)
		}

		// Listen for progress
		go func() {
			for progress := range downloader.GetProgressChannel() {
				if progress.ChapterNumber != "" {
					if progress.Status == "complete" {
						fmt.Printf("  ✓ Chapter %s complete\n", progress.ChapterNumber)
					} else if progress.TotalPages > 0 {
						fmt.Printf("  Chapter %s: %d/%d pages\n", progress.ChapterNumber, progress.CurrentPage, progress.TotalPages)
					} else if progress.Status == "error" {
						fmt.Printf("  ✗ Chapter %s error: %v\n", progress.ChapterNumber, progress.Error)
					}
				}
			}
		}()

		if err := downloader.DownloadManga(manga, filteredChapters); err != nil {
			cobra.CheckErr(fmt.Errorf("download failed: %w", err))
		}

		fmt.Println("\n✅ Download complete! EPUBs have been created in:", downloadDir)
	},
}

func init() {
	downloadCmd.Flags().StringP("language", "l", "en", "Language code (e.g., en, ja, es)")
	downloadCmd.Flags().StringP("chapters", "c", "", "Chapter range (e.g., 1-10)")
}
