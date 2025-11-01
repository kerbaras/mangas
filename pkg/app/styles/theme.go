package styles

import "github.com/charmbracelet/lipgloss"

var (
	// Color palette
	Primary   = lipgloss.Color("#FF6B9D")
	Secondary = lipgloss.Color("#C792EA")
	Success   = lipgloss.Color("#C3E88D")
	Warning   = lipgloss.Color("#FFCB6B")
	Error     = lipgloss.Color("#F07178")
	Info      = lipgloss.Color("#82AAFF")
	Muted     = lipgloss.Color("#546E7A")
	Background = lipgloss.Color("#263238")
	Foreground = lipgloss.Color("#EEFFFF")
	
	// Border styles
	RoundedBorder = lipgloss.RoundedBorder()
	ThickBorder   = lipgloss.ThickBorder()
)

// Base styles
var (
	// Title style for headings
	TitleStyle = lipgloss.NewStyle().
		Foreground(Primary).
		Bold(true).
		MarginBottom(1)
	
	// Subtitle style
	SubtitleStyle = lipgloss.NewStyle().
		Foreground(Secondary).
		Italic(true)
	
	// Normal text
	TextStyle = lipgloss.NewStyle().
		Foreground(Foreground)
	
	// Muted/dimmed text
	MutedStyle = lipgloss.NewStyle().
		Foreground(Muted)
	
	// Selected item
	SelectedStyle = lipgloss.NewStyle().
		Foreground(Primary).
		Bold(true).
		BorderStyle(RoundedBorder).
		BorderForeground(Primary).
		Padding(0, 1)
	
	// Card style
	CardStyle = lipgloss.NewStyle().
		Border(RoundedBorder).
		BorderForeground(Secondary).
		Padding(1, 2).
		MarginBottom(1)
	
	// Active/focused card
	ActiveCardStyle = lipgloss.NewStyle().
		Border(ThickBorder).
		BorderForeground(Primary).
		Padding(1, 2).
		MarginBottom(1)
	
	// Status styles
	StatusDownloading = lipgloss.NewStyle().
		Foreground(Info).
		Bold(true)
	
	StatusCompleted = lipgloss.NewStyle().
		Foreground(Success).
		Bold(true)
	
	StatusError = lipgloss.NewStyle().
		Foreground(Error).
		Bold(true)
	
	// Progress bar styles
	ProgressBarStyle = lipgloss.NewStyle().
		Foreground(Primary)
	
	ProgressEmptyStyle = lipgloss.NewStyle().
		Foreground(Muted)
	
	// Tab styles
	ActiveTabStyle = lipgloss.NewStyle().
		Foreground(Primary).
		Background(lipgloss.Color("#37474F")).
		Padding(0, 2).
		Bold(true)
	
	InactiveTabStyle = lipgloss.NewStyle().
		Foreground(Muted).
		Padding(0, 2)
	
	// Help text
	HelpStyle = lipgloss.NewStyle().
		Foreground(Muted).
		Italic(true).
		MarginTop(1)
	
	// Input field
	InputStyle = lipgloss.NewStyle().
		Border(RoundedBorder).
		BorderForeground(Secondary).
		Padding(0, 1)
	
	// Focused input
	FocusedInputStyle = lipgloss.NewStyle().
		Border(RoundedBorder).
		BorderForeground(Primary).
		Padding(0, 1)
)

// Helper functions
func StatusStyle(status string) lipgloss.Style {
	switch status {
	case "downloading", "processing":
		return StatusDownloading
	case "completed", "complete":
		return StatusCompleted
	case "error", "partial":
		return StatusError
	default:
		return MutedStyle
	}
}
