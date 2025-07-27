package main

import (
	"log"

	"github.com/kerbaras/mangas/pkg/app"
	"github.com/spf13/cobra"
)

var rootCmd = &cobra.Command{
	Use:   "mangas",
	Short: "A beautiful manga bookshelf CLI",
	Long:  "Manage your manga collection with style",
	Run: func(cmd *cobra.Command, args []string) {
		app := app.NewApp()
		app.Run()
	},
}

func init() {
	// cobra.OnInitialize(initConfig)
}

func main() {
	if err := rootCmd.Execute(); err != nil {
		log.Fatal(err)
	}
}
