package cmd

import (
	"fmt"

	"github.com/spf13/cobra"
)

var epubCmd = &cobra.Command{
	Use:   "epub [manga-id]",
	Short: "Generate EPUB from downloaded chapters (deprecated)",
	Long:  "This command is deprecated. EPUBs are now generated automatically during chapter download.",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		fmt.Println("ℹ️  This command is deprecated.")
		fmt.Println("📖 EPUBs are now generated automatically during chapter download.")
		fmt.Println("💡 Use 'mangas download' to download chapters and create EPUBs in one step.")
	},
}
