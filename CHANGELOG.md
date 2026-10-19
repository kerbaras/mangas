# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
## [0.4.0] - 2026-10-19

### Added

- **providers:** Share per-host rate limits across processes (#120)
- **export:** Volume bundling with EPUB TOC, series metadata and cover page (#122)
- **export:** Exporter trait with EPUB, CBZ and PDF, e-reader profiles
- **export:** Send to Kindle by email or USB
- **providers:** Season-aware display labels for Webtoons episodes
- **desktop:** One export dialog for chapters, volumes, formats and delivery
- **core:** Name per-chapter export books after their chapter
- **export:** Smart-cut preview and per-series export presets
- **desktop:** Series preset in the unified export dialog, wait for detectors before exporting
- **web:** Landing page on GitHub Pages (#169)
- **providers:** Map MangaDot groups and add a per-series group filter (#162)
- **desktop:** New app icon (open book + ribbon) (#168)
- **ml:** Ship detector support in every release via on-demand ONNX Runtime (#163)
- **web:** Landing page v2 with real app screenshots and scroll-driven motion (#204)

### Changed

- **core:** Create the log folder in core

### Fixed

- **providers:** Follow redirects hop by hop with each host's limiter (#118)
- **download:** Name page files by their real image format (#117)
- **providers:** Resolve-time public IP check, https upgrade, covers on PageClient (#119)
- **providers:** Refuse private IP literals and localhost as BlockedAddress (#140)
- **cli:** Kindle keeps writing a Paperwhite EPUB by default; volume export uses the saved profile
- **providers:** Warn once per host when the shared rate limit file is unusable
- **download:** Save a job's page count on every page landed
- **download:** Wake the worker on every landed page
- **cli:** Title a chapter-folder export from the library's series
- **cli:** A mixed chapter export keeps its first chapter's title
- **core:** Validate imported page images, name a lone CBZ after its file, report skip reasons
- **core:** Decode the whole page in the import check
- **desktop:** Keep the skipped list inside the import toast
- **download:** Clean stale .part files when a chapter completes
- **download:** Stray .part cleanup is best-effort; keep a symlinked old data folder
- **download:** Remove .part files by their real name, even when it is not UTF-8
- **download:** Try every stale .part file, warning per one that stays
- **send:** End the USB progress line before errors, skip it for books already on the Kindle, and disable Email for CBZ
- **desktop:** No empty copying state before the first progress event; focusable reason for disabled Email
- **desktop:** Show the copying state as soon as the Kindle is found
- **download:** A chapter folder missing pages is downloaded again, not skipped
- **download:** Record the page count before fetching and check every page index
- **download:** Replace the page-count marker by rename
- **download:** Give each marker write its own aside file
- **download:** Repair folders without a page-count marker and sweep stray marker files
- **desktop:** Tag only season episodes in export names; split season provider test
- **core:** Name season chapters by their label in volume exports and TOCs
- **core:** Keep part numbers and leading spaces in season tags
- **core:** Name plain chapters ch 12 in volume exports too
- **desktop:** Grant opener open_path so Open folder works
- **desktop:** Open folders through a checked open_folder command
- **desktop:** Create the log folder before offering to open it
- **desktop:** Grant the notification permission check and guard every UI plugin
- **core:** Open recorded export folders only, not their subfolders
- **desktop:** Export the hidden selection and count skipped files across books
- **desktop:** Preview the names an export really writes, keep partial sends, explain disabled email, clear stale results
- **desktop:** Clear results on cut changes and after a busy change, match names case-insensitively
- **core:** Export into folders that can be written but not listed
- **desktop:** Wait for the series preset before exporting
- **desktop:** Keep the window from scrolling so macOS overscroll stays in the pane (#203)
- **ml:** Load the ONNX Runtime on Apple Silicon and cut without detectors when it can't (#205)
## [0.4.0-rc.2] - 2026-10-19

### Added

- **web:** Landing page v2 with real app screenshots and scroll-driven motion (#204)

### Fixed

- **desktop:** Keep the window from scrolling so macOS overscroll stays in the pane (#203)
- **ml:** Load the ONNX Runtime on Apple Silicon and cut without detectors when it can't (#205)
## [0.4.0-rc.1] - 2026-10-18

### Added

- **providers:** Share per-host rate limits across processes (#120)
- **export:** Volume bundling with EPUB TOC, series metadata and cover page (#122)
- **export:** Exporter trait with EPUB, CBZ and PDF, e-reader profiles
- **export:** Send to Kindle by email or USB
- **providers:** Season-aware display labels for Webtoons episodes
- **desktop:** One export dialog for chapters, volumes, formats and delivery
- **core:** Name per-chapter export books after their chapter
- **export:** Smart-cut preview and per-series export presets
- **desktop:** Series preset in the unified export dialog, wait for detectors before exporting
- **web:** Landing page on GitHub Pages (#169)
- **providers:** Map MangaDot groups and add a per-series group filter (#162)
- **desktop:** New app icon (open book + ribbon) (#168)
- **ml:** Ship detector support in every release via on-demand ONNX Runtime (#163)

### Changed

- **core:** Create the log folder in core

### Fixed

- **providers:** Follow redirects hop by hop with each host's limiter (#118)
- **download:** Name page files by their real image format (#117)
- **providers:** Resolve-time public IP check, https upgrade, covers on PageClient (#119)
- **providers:** Refuse private IP literals and localhost as BlockedAddress (#140)
- **cli:** Kindle keeps writing a Paperwhite EPUB by default; volume export uses the saved profile
- **providers:** Warn once per host when the shared rate limit file is unusable
- **download:** Save a job's page count on every page landed
- **download:** Wake the worker on every landed page
- **cli:** Title a chapter-folder export from the library's series
- **cli:** A mixed chapter export keeps its first chapter's title
- **core:** Validate imported page images, name a lone CBZ after its file, report skip reasons
- **core:** Decode the whole page in the import check
- **desktop:** Keep the skipped list inside the import toast
- **download:** Clean stale .part files when a chapter completes
- **download:** Stray .part cleanup is best-effort; keep a symlinked old data folder
- **download:** Remove .part files by their real name, even when it is not UTF-8
- **download:** Try every stale .part file, warning per one that stays
- **send:** End the USB progress line before errors, skip it for books already on the Kindle, and disable Email for CBZ
- **desktop:** No empty copying state before the first progress event; focusable reason for disabled Email
- **desktop:** Show the copying state as soon as the Kindle is found
- **download:** A chapter folder missing pages is downloaded again, not skipped
- **download:** Record the page count before fetching and check every page index
- **download:** Replace the page-count marker by rename
- **download:** Give each marker write its own aside file
- **download:** Repair folders without a page-count marker and sweep stray marker files
- **desktop:** Tag only season episodes in export names; split season provider test
- **core:** Name season chapters by their label in volume exports and TOCs
- **core:** Keep part numbers and leading spaces in season tags
- **core:** Name plain chapters ch 12 in volume exports too
- **desktop:** Grant opener open_path so Open folder works
- **desktop:** Open folders through a checked open_folder command
- **desktop:** Create the log folder before offering to open it
- **desktop:** Grant the notification permission check and guard every UI plugin
- **core:** Open recorded export folders only, not their subfolders
- **desktop:** Export the hidden selection and count skipped files across books
- **desktop:** Preview the names an export really writes, keep partial sends, explain disabled email, clear stale results
- **desktop:** Clear results on cut changes and after a busy change, match names case-insensitively
- **core:** Export into folders that can be written but not listed
- **desktop:** Wait for the series preset before exporting
## [0.3.0] - 2026-10-18

### Added

- Storage management and library backup/restore (#100)
- **library:** Categories, reading status, sort/filter and chapter state (#103)
- **download:** Persistent download queue with concurrency caps, pause/resume/cancel and retries (#101)
- **library:** New-chapter checks, badges, auto-download and notifications (#106)
- **desktop:** Downloads screen with queue sections, gate banner and nav badge (#110)

### Fixed

- **download:** Don't settle a CLI run while another process still downloads its chapter (#109)
- **download:** Expire stale running-job leases and reclaim orphans on startup (#111)
- **backup:** Carry categories, reading status and read marks in backups (#113)
- **settings:** Reload on focus, merge stale desktop edits, choose restore preferences (#114)
## [0.2.0] - 2026-10-18

### Added

- **providers:** WeebCentral provider on a shared html helper (#67)
- **providers:** Add Tapas provider (free episodes) (#65)
- **providers:** Add Comick provider (comick.live, session-aware search) (#66)
- **providers:** Webtoons (LINE) provider for originals and canvas (#69)
- **search:** Multi-provider search and provider management (#78)
- List locked and externally hosted chapters with an availability (#84)
- **desktop:** Fetch covers through the provider and cache them on disk (#77)
- **cli:** Provider options, preferred languages and multi-language sync (#83)
- Authors, artists, year and tags in manga details (#86)

### Fixed

- **providers:** Share one rate limiter per host across clients and calls (#64)
- **cli:** Exit quietly when stdout is closed (broken pipe) (#79)
- **cli:** Plain log output when stderr is not a terminal (#81)
- **providers:** Name skipped check results, flag skipped live tests, redact debug query values (#85)
- **desktop:** Load the stored manga on the details screen instead of the search card (#82)
- Clean up failed local imports and show plain import errors in a toast (#89)
- **desktop:** Log every Tauri command error once, under the command's name (#87)
- **tests:** Default the new Manga metadata fields in test fixtures (#96)
- **download:** Fetch pages through the provider's rate limit and retry policy (#90)
- **providers:** Send the site referer and same-origin fetch headers on API requests (#95)
- **desktop:** Settings load error with retry, search kept on back, chapter download failures logged with Connect (#98)
- Provider text polish and regional language fallback in search and details (#97)
## [0.1.0] - 2026-10-18

### Added

- **library:** Import local chapter folders and CBZ files (#47)
- **providers:** Provider harness with SiteClient, host policy, retries and status (#48)
- **diagnostics:** Add file logging, diagnostics and error surfaces (#46)
- **providers:** Provider contract-test suite and providers check (#53)
- **providers:** MangaDex covers, content rating, groups, languages and data-saver (#54)

### Fixed

- **core:** Export and import GIF pages, warn on skipped files (#59)
- **providers:** Retry MangaDex edge HTML 400 pages on feed requests (#62)
## [0.1.0-rc.6] - 2026-10-16

### Fixed

- **desktop:** Connect window user agent matches its engine and keeps no cookies
## [0.1.0-rc.5] - 2026-10-16

### Fixed

- **desktop:** Capture parent-domain cookies when saving a provider session on macOS
- **desktop:** Ignore cookies without a domain when capturing a provider session
## [0.1.0-rc.4] - 2026-10-16

### Fixed

- **desktop:** Ad-hoc sign the macOS bundle and keep the traffic lights inside the nav rail
## [0.1.0-rc.1] - 2026-10-16

### Added

- **sources:** Add MangaDex source implementation and corresponding tests
- Implement CLI commands for manga management, including search, add, download, and EPUB generation; add Makefile for build automation and testing; enhance documentation and coverage ignore settings.
- Update dependencies and enhance manga download functionality; add Kindle export feature with optimized image processing; deprecate manual EPUB generation command.
- Rust workspace scaffold for the Tauri rewrite
- Rust + Tauri workspace scaffold (rewrite PR 1) (#3)
- **desktop:** Library, search, details, Kindle export and settings screens with mock IPC
- Integrate sources, Kindle export and desktop UI; remove the Go implementation (#7)
- **providers:** Packages/providers crate with MangaDex and MangaDot, browser-session bridge for Cloudflare-protected sites
- **mangadot:** Status from the page loader data; connect window navigation pinned to the provider origin

### Fixed

- **core:** Open bare db filenames, order library by insertion, pin model sha256
- **desktop:** Review findings
- **desktop:** Ignore failed downloads when refetching chapters; clear follow error on new query
- Post-merge review of the rewrite (unfollowed chapters, default data dir, reserved IPv4) (#8)
- **cli:** Share the app's data folder; install_cli links the AppImage file and only replaces its own link
- **cli:** Fall back to the platform data dir when the library is unreadable; tighten install_cli ownership check
- **providers:** Keep sessions on the site host only, backend-owned sessions, majority check for the search endpoint, poll for the captured session
- **desktop:** A reconnect waits for a session newer than the one it started with
- **mangadot:** Search text parameter named like the site's own search route (search=)
- **desktop:** Connect window navigation must stay on https
[0.4.0-rc.2]: https://github.com/kerbaras/mangas/compare/v0.4.0-rc.1...v0.4.0-rc.2
[0.4.0-rc.1]: https://github.com/kerbaras/mangas/compare/v0.3.0...v0.4.0-rc.1
[0.3.0]: https://github.com/kerbaras/mangas/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/kerbaras/mangas/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/kerbaras/mangas/compare/v0.1.0-rc.6...v0.1.0
[0.1.0-rc.6]: https://github.com/kerbaras/mangas/compare/v0.1.0-rc.5...v0.1.0-rc.6
[0.1.0-rc.5]: https://github.com/kerbaras/mangas/compare/v0.1.0-rc.4...v0.1.0-rc.5
[0.1.0-rc.4]: https://github.com/kerbaras/mangas/compare/v0.1.0-rc.3...v0.1.0-rc.4
[0.1.0-rc.1]: https://github.com/kerbaras/mangas/releases/tag/v0.1.0-rc.1

