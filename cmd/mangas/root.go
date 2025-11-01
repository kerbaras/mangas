package cmd

import (
	"os"

	"github.com/kerbaras/mangas/pkg/app"
	"github.com/spf13/cobra"
)

var rootCmd = &cobra.Command{
	Use:   "mangas",
	Short: "A beautiful manga bookshelf CLI",
	Long:  "Download and manage your manga collection with a beautiful TUI and CLI",
	Run: func(cmd *cobra.Command, args []string) {
		// Launch TUI by default
		a := app.NewApp()
		if err := a.Run(); err != nil {
			cobra.CheckErr(err)
		}
	},
}

func init() {
	// Add all subcommands
	rootCmd.AddCommand(listCmd)
	rootCmd.AddCommand(downloadCmd)
	rootCmd.AddCommand(epubCmd)
}

func Execute() {
	if err := rootCmd.Execute(); err != nil {
		os.Exit(1)
	}
}
