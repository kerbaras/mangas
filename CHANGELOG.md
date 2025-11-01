# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Added - CLI Commands Implementation

#### New Commands
- `mangas` - Launch interactive TUI (default behavior)
- `mangas search <query>` - Search for manga on MangaDex with formatted table output
- `mangas add <manga-name>` - Add manga to library (metadata only)
- `mangas list` - Display library contents in formatted table
- `mangas download <manga-name>` - Enhanced download with name-based lookup
  - `--language` flag for language selection (default: en)
  - `--chapters` flag for chapter range (e.g., 1-10)
- `mangas epub <manga-id>` - Generate EPUB from downloaded chapters

#### Features
- **Table Formatting**: Beautiful table output using text/tabwriter
- **Smart Lookup**: Download by manga name (searches library) or direct ID
- **Progress Tracking**: Real-time download progress in CLI
- **Helpful Tips**: Context-aware suggestions after each command
- **Comprehensive Help**: Detailed help text for all commands

#### Documentation
- **CLI.md**: Complete CLI reference with examples and workflows
- **Updated README.md**: New usage examples and command overview
- All commands have `--help` support

### Technical Details
- Replaced tablewriter external dependency with stdlib text/tabwriter
- Added name-to-ID resolution for downloads
- Improved error messages and user guidance
- Maintained backward compatibility with existing commands

### Previous Features

#### Testing (v0.2.0)
- **81.5% test coverage** for core business logic
- 60+ test cases across 9 test files
- Mock objects for external dependencies
- Comprehensive TESTING.md documentation
- Makefile for easy testing
- GitHub Actions workflow

#### Core Functionality (v0.1.0)
- Beautiful TUI with Bubble Tea framework
- MangaDex API integration
- Complete download orchestration
- EPUB generation for entire manga series
- DuckDB for local library management
- Progress tracking with real-time updates

## Command Examples

### Workflow Example
```bash
# Search for manga
$ mangas search "One Punch Man"

# Add to library
$ mangas add "One Punch Man"

# Download chapters
$ mangas download "One Punch Man" --language en

# View library
$ mangas list

# Launch TUI for browsing
$ mangas
```

### Advanced Usage
```bash
# Download specific chapters (framework ready)
$ mangas download "Naruto" --language en --chapters 1-10

# Download in different language
$ mangas download "Naruto" --language ja

# Download by ID
$ mangas download a1c7c817-4e59-43b7-9365-09675a149a6f

# Regenerate EPUB
$ mangas epub a1c7c817-4e59-43b7-9365-09675a149a6f
```

## Future Roadmap

- [ ] Implement chapter range filtering in download
- [ ] Resume interrupted downloads
- [ ] Batch download operations
- [ ] Configuration file support
- [ ] Additional manga sources
- [ ] Cover image download
- [ ] Reading progress tracking
- [ ] Web UI interface

---

See [README.md](README.md) for installation and general usage.
See [CLI.md](CLI.md) for complete CLI reference.
See [TESTING.md](TESTING.md) for testing documentation.
