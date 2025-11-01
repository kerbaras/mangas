package cmd

import (
	"fmt"
	"log"
	"strings"

	"github.com/kerbaras/mangas/pkg/data"
	"github.com/kerbaras/mangas/pkg/sources"
	"github.com/spf13/cobra"
)

var addCmd = &cobra.Command{
	Use:   "add [manga-name]",
	Short: "Add a manga to your library",
	Long:  "Search for a manga and add it to your library (downloads metadata only)",
	Args:  cobra.MinimumNArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		query := strings.Join(args, " ")
		source := sources.NewMangaDex()
		repo := data.NewDuckDBRepository()

		fmt.Printf("🔍 Searching for '%s'...\n", query)

		results, err := source.Search(query)
		if err != nil {
			cobra.CheckErr(fmt.Errorf("search failed: %w", err))
		}

		if len(results) == 0 {
			fmt.Println("❌ No results found.")
			return
		}

		// Take the first result
		manga := results[0]
		fmt.Printf("✅ Found: %s (ID: %s)\n", manga.Name, manga.ID)

		// Get chapters to count them
		chapters, err := source.GetChapters(&manga)
		if err != nil {
			cobra.CheckErr(fmt.Errorf("failed to get chapters: %w", err))
		}

		// Save manga to database
		if err := repo.SaveManga(&manga); err != nil {
			cobra.CheckErr(fmt.Errorf("failed to save manga: %w", err))
		}

		// Save chapter metadata (not downloaded yet)
		for i := range chapters {
			chapters[i].MangaID = manga.ID
			if err := repo.SaveChapter(&chapters[i]); err != nil {
				log.Printf("Warning: Failed to save chapter %s: %v", chapters[i].Number, err)
			}
		}

		fmt.Printf("✅ Added '%s' to library with %d chapters\n", manga.Name, len(chapters))
		fmt.Printf("💡 To download chapters, use: mangas download \"%s\" --language en\n", manga.Name)
	},
}

func init() {
	addCmd.Flags().StringP("language", "l", "en", "Language of the manga")

	rootCmd.AddCommand(addCmd)
}
