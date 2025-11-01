# AGENTS.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Go-based manga bookshelf CLI application built with Bubble Tea TUI framework. The application allows users to search, download, and read manga from various sources (primarily MangaDx) with local storage in CBZ/EPUB formats and DuckDB for metadata.

## Commands

### Build and Run
```bash
make build
./bin/mangas
```

Or directly:
```bash
go build -o bin/mangas ./cmd
./bin/mangas
```

### Development
```bash
go run ./cmd
```

### Testing
```bash
go test ./...
```

### Dependencies
```bash
go mod tidy
go mod download
```

## Architecture

The codebase follows a layered architecture:

- **UI Layer** (`pkg/app/`): Bubble Tea TUI components and screens
- **Business Logic** (`pkg/services/`): Core application logic and downloaders
- **Data Layer** (`pkg/data/`): DuckDB integration and data models
- **Sources** (`pkg/sources/`): Manga source integrations (MangaDx, Local)
- **Integrations** (`pkg/integrations/`): File format processors (CBZ, EPUB)

### Key Interfaces

- `sources.Source`: Defines manga source operations (search, download)
- `integrations.Processor`: Handles manga chapter processing to different formats

### Entry Points

- `cmd/main.go`: Main entry point
- `cmd/mangas/root.go`: Cobra CLI root command and TUI launcher
- `pkg/app/app.go`: Bubble Tea application setup
- `pkg/app/screens/root.go`: Main TUI screen

### CLI Commands

All CLI commands are in `cmd/mangas/`:
- `root.go` - Root command (launches TUI by default)
- `list.go` - List library (uses bubbles/table)
- `search.go` - Search MangaDex (uses bubbles/table)
- `add.go` - Add manga to library
- `download.go` - Download manga chapters
- `epub.go` - Generate EPUB files
- `helpers.go` - Shared utility functions

### Data Models

Core entities are defined in `pkg/data/model.go`:
- `Manga`: Basic manga metadata
- `Chapter`: Chapter information

### TUI Structure

- **Screens** (`pkg/app/screens/`): Different application views (library, search, details)
- **Components** (`pkg/app/components/`): Reusable UI components (filter, manga list, progress)
- **Styles** (`pkg/app/styles/`): Theme and styling definitions

## Key Dependencies

### Charm Bracelet Suite
- `github.com/charmbracelet/bubbletea`: TUI framework
- `github.com/charmbracelet/lipgloss`: Terminal styling
- `github.com/charmbracelet/bubbles/table`: Table component for CLI
- `github.com/charmbracelet/bubbles/textinput`: Text input component

### Core Dependencies
- `github.com/marcboeker/go-duckdb/v2`: Database integration
- `github.com/spf13/cobra`: CLI framework
- `github.com/go-shiori/go-epub`: EPUB generation