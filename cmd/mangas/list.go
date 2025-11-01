package cmd

import (
	"fmt"

	"github.com/charmbracelet/bubbles/table"
	"github.com/charmbracelet/lipgloss"
	"github.com/kerbaras/mangas/pkg/data"
	"github.com/spf13/cobra"
)

var listCmd = &cobra.Command{
	Use:   "list",
	Short: "List all manga in your library",
	Long:  "Display all manga in your library in a formatted table",
	Run: func(cmd *cobra.Command, args []string) {
		repo := data.NewDuckDBRepository()
		mangas, err := repo.ListMangas()
		if err != nil {
			cobra.CheckErr(err)
		}

		if len(mangas) == 0 {
			fmt.Println("📚 No manga in library. Use 'mangas search' to find manga to add.")
			return
		}

		// Create table columns
		columns := []table.Column{
			{Title: "Name", Width: 40},
			{Title: "Source", Width: 10},
			{Title: "Status", Width: 12},
			{Title: "Chapters", Width: 10},
			{Title: "Downloaded", Width: 12},
		}

		rows := []table.Row{}
		for _, manga := range mangas {
			_, total, downloaded, _ := repo.GetMangaWithChapterCount(manga.ID)
			status := manga.Status
			if status == "" {
				status = "ready"
			}

			rows = append(rows, table.Row{
				truncateString(manga.Name, 38),
				manga.Source,
				status,
				fmt.Sprintf("%d", total),
				fmt.Sprintf("%d", downloaded),
			})
		}

		t := table.New(
			table.WithColumns(columns),
			table.WithRows(rows),
			table.WithFocused(false),
			table.WithHeight(len(rows)),
		)

		s := table.DefaultStyles()
		s.Header = s.Header.
			BorderStyle(lipgloss.NormalBorder()).
			BorderForeground(lipgloss.Color("240")).
			BorderBottom(true).
			Bold(true)
		s.Selected = s.Selected.
			Foreground(lipgloss.Color("229")).
			Background(lipgloss.Color("57")).
			Bold(false)
		t.SetStyles(s)

		fmt.Printf("\n📚 Library (%d manga)\n\n", len(mangas))
		fmt.Println(t.View())
	},
}
