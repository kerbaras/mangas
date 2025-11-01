package cmd

import (
	"fmt"
	"os"
	"path/filepath"

	"github.com/kerbaras/mangas/pkg/data"
	"github.com/kerbaras/mangas/pkg/integrations"
	"github.com/kerbaras/mangas/pkg/services"
	"github.com/kerbaras/mangas/pkg/sources"
	"github.com/spf13/cobra"
)

var epubCmd = &cobra.Command{
	Use:   "epub [manga-id]",
	Short: "Generate EPUB from downloaded chapters",
	Long:  "Compile all downloaded chapters of a manga into a single EPUB file",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		mangaID := args[0]

		repo := data.NewDuckDBRepository()
		source := sources.NewMangaDex()

		homeDir, _ := os.UserHomeDir()
		downloadDir := filepath.Join(homeDir, ".mangas", "downloads")
		epubDir := filepath.Join(homeDir, ".mangas", "library")

		processor := integrations.NewEPUBProcessor(epubDir)
		downloader := services.NewDownloader(source, repo, processor, downloadDir)

		fmt.Println("📖 Generating EPUB...")
		epubPath, err := downloader.ComposeEPUB(mangaID)
		if err != nil {
			cobra.CheckErr(fmt.Errorf("EPUB generation failed: %w", err))
		}

		fmt.Printf("✅ EPUB created: %s\n", epubPath)
	},
}
