# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
## [0.6.0-rc.1] - 2026-10-22

### Added

- **export:** Warn over the Send-to-Kindle limit, split into parts, JPEG quality per profile (#287)
- **providers:** Add MANGA Plus (protobuf API, XOR-encrypted pages) (#308)
- Rename the product from Mangas to mangas (#294)
- **send:** Detect a Kindle connected over MTP only and point to Email (#371)
- **desktop:** English message catalog and useT hook for library, details and search (#378)

### Changed

- **export:** Decode and encode regular pages in parallel, three at a time (#277)
- **desktop:** Progress bars step about 4 times a second, with no layout or transition (#288)
- **desktop:** Still icons for open-ended waits instead of endless spinners (#273)
- **export:** Stream pages through an on-disk spool into EPUB, CBZ and PDF (#281)
- **core:** Count Library chapters with one aggregate query (#349)
- **desktop:** Window the Details chapter list (#350)
- **desktop:** Page the Completed downloads list (#352)
- **desktop:** Still indicators instead of a pulsing progress bar and endless spinners (#356)

### Fixed

- **desktop:** Keep a finished delivery's result when the tab changed mid-run (#272)
- **desktop:** Try again after a failed export exports again (#274)
- **desktop:** Offer Remove on the detector card when a remove left files behind (#278)
- **desktop:** Point detector support at Settings → Detectors (#275)
- **desktop:** Keep a long placeholder cover title inside the cover (#276)
- **desktop:** End the Connect wait when its window closes or its page fails to load (#269)
- **desktop:** Valid tab aria-controls and a Downloads rail name with its count (#279)
- **desktop:** Pass WCAG AA contrast for captions, the primary button, warn and the nav badge (#282)
- **export:** Deterministic EPUB modified date, no stray ZipWriter::drop log on a failed write (#270)
- **export:** A failed volume stops the run, keeps the books before it and reports both (#271)
- **storage:** Refuse a data-folder move while an export reads chapters (#280)
- **smartcut:** Skip long blank gutters, drop blank pages, keep tall balloons whole (#292)
- **providers:** Let the proxy resolve page hosts when one is configured (#305)
- **cli:** Resolve a title's followed source when --source is left out (#306)
- **kindle:** Name a moved chapter folder's book from the library, never the manga id (#307)
- **export:** Plan the names a write-only folder really gives in --dry-run and the preview (#334)
- **cli:** Print each error reason once (#304)
- **storage:** A refused data-folder move leaves no target folder behind (#354)
- **storage:** Reusing a partial copy deletes only the files the move copied (#353)
- **desktop:** Hold the export lease while a smart-cut preview reads the chapter (#344)
- **ml:** A spawned status child never holds a lease of the app (#355)
- **desktop:** Replace an earlier error toast when the same action later succeeds (#342)
- **desktop:** Show an export failure once, in the dialog's panel (#363)
- **desktop:** Open the folder pickers at the folder they change (#375)
- **storage:** Take the export lease for Delete downloads in the app and the CLI (#345)
- **export:** Valid Kindle writing mode and the series' reading direction (#358)
- **cli:** Record kindle exports from moved or copied chapter folders (#359)
- **cli:** Print the partial report when kindle --json fails (#360)
- **send:** Plain not-enough-space error for a full Kindle over USB (#361)
- **export:** Name the book only by renaming its finished part (#347)
- **cli,desktop:** Update checks name the network cause (Connection refused, DNS), once (#348)
- **providers:** Show MangaDot's PNG icon, loaded without a Referer (#362)
- **providers,core:** Page hosts that can't be looked up fail after one retry with the cause (#351)
- **cli,desktop:** A stored non-public-address refusal offers Retry once a proxy is in use (#357)
- **desktop:** Name the certificate problem instead of "Couldn't reach the site" (#340)
- **desktop:** Keep the blocked Email delivery option a tab, so the tablist is axe clean (#377)
- **desktop:** Give the Library Cover header text and the Search provider groups h2 (#388)
- **desktop:** Try again after a stopped export writes only the books it didn't (#365)
- **desktop:** End the Connect wait on a failed load on macOS and Windows too (#341)
- **core:** Io errors name the file or folder they failed on, once (#364)
- **desktop:** Io hints name the export or data folder that failed (#376)
## [0.5.0] - 2026-10-20

### Added

- **settings:** Grouped settings with theme, download concurrency, HTTP(S) proxy and per-group reset (#182)
- **desktop:** Auto-updater with stable and rc channels (#176)
- **cli:** Add --json to mangas chapters (#212)
- **desktop:** First-run setup, keyboard shortcuts and remembered window state (#175)
- **export:** Add a Replace existing files option to the writers, CLI and Export dialog (#217)
- **cli:** Add --strict-detectors to kindle and export (#215)
- **send:** Keep the SMTP password in the OS keychain (#174)
- **cli:** --json report for kindle and export with the detectors state (#232)

### Fixed

- **cli:** Size the chapters id column to the longest id and pad ids like strings (#173)
- **send:** Give the SMTP handshake its own deadline and cap the upload allowance (#180)
- Show the release's group on queue rows and count only the saved group's chapters (#181)
- **providers:** Give MangaDot's whole chapter list its own 90 s timeout (#185)
- **desktop:** Show chapter failure reasons in an edge-aware tooltip (#177)
- Polish refused-download messages (no retry hint, word wrap, IPv4-mapped display) (#178)
- **providers:** Stop waiting on a held rate limit lock after 2 s and pace in memory (#216)
- Depend on ca-certificates in Linux packages and name missing root certificates (#213)
- **desktop:** Show the window once the page has its palette (#233)
- **desktop:** Keep the normal window geometry apart from the maximised flag (#230)
- **desktop:** Keep Library and Search scroll and filters across Back (#229)
- **desktop:** Replace native title tooltips with the shared Hint (#228)
- **desktop:** Readable accent text token for dark mode (#224)
- **export:** Shorten file names of long titles in core (#225)
- **cli:** Write kindle -o <folder> inside the folder (#226)
- **cli:** Providers exits 1 when every provider is unreachable, add --json (#227)
- **desktop:** Keep screen header actions on one row at 960 px (#241)
- **desktop:** Show a truncated screen header title in the shared Hint (#243)
- **download:** Refuse re-downloading imported chapters, offer Open folder (#250)
- **storage:** Mark data-folder moves so a killed move's partial copy can be listed, removed or replaced (#256)
- **desktop:** Run detector removal and folder-heavy commands off the UI thread (#255)
- **storage:** Forget the runtime failure when Storage deletes detector support (#246)
- **cli:** Size providers table columns to the longest row (#242)
- **desktop:** Add partial_move to the detector-remove-refresh fixture (#263)
- **export:** Retry after a failed delivery re-sends the written book instead of re-exporting (#252)
- **settings:** Keep the SMTP password out of get_settings; write-only field with set_smtp_password (#249)
- **detectors:** Check detector status in a child process so Windows can remove the runtime (#257)
- **export:** Resolve the series cover under the settings fallback (#251)
- **desktop:** Show long Storage titles and the delete label in the shared Hint (#267)
- **desktop:** Keep the Smart-cut options button on one line (#268)
- **desktop:** Wrap unbroken tokens in plain error and note lines (#266)
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

