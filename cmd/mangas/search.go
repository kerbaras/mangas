package cmd

import (
	"fmt"
	"strings"

	"github.com/charmbracelet/lipgloss"
	"github.com/charmbracelet/lipgloss/table"
	"github.com/kerbaras/mangas/pkg/sources"
	"github.com/spf13/cobra"
)

var searchCmd = &cobra.Command{
	Use:   "search [query]",
	Short: "Search for manga",
	Long:  "Search for manga on MangaDex and display results in a table",
	Args:  cobra.MinimumNArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		query := strings.Join(args, " ")
		source := sources.NewMangaDex()

		results, err := source.Search(query)
		if err != nil {
			cobra.CheckErr(fmt.Errorf("search failed: %w", err))
		}

		if len(results) == 0 {
			fmt.Println("No results found.")
			return
		}

		var (
			purple = lipgloss.Color("99")

			headerStyle = lipgloss.NewStyle().Foreground(purple).Bold(true).Align(lipgloss.Center)
			cellStyle   = lipgloss.NewStyle().Padding(0, 1)
		)

		t := table.New().
			Border(lipgloss.HiddenBorder()).
			BorderStyle(lipgloss.NewStyle().Foreground(purple)).
			StyleFunc(func(row, col int) lipgloss.Style {
				switch {
				case row == table.HeaderRow:
					return headerStyle
				default:
					return cellStyle
				}
			}).
			Headers("#", "Name", "ID")

		for i, manga := range results {
			t.Row(fmt.Sprintf("%d", i+1), truncateString(manga.Name, 58), manga.ID)
		}

		fmt.Println(t)
	},
}

func init() {
	rootCmd.AddCommand(searchCmd)
}
