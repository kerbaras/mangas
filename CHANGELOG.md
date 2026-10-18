# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
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
[0.2.0]: https://github.com/kerbaras/mangas/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/kerbaras/mangas/compare/v0.1.0-rc.6...v0.1.0
[0.1.0-rc.6]: https://github.com/kerbaras/mangas/compare/v0.1.0-rc.5...v0.1.0-rc.6
[0.1.0-rc.5]: https://github.com/kerbaras/mangas/compare/v0.1.0-rc.4...v0.1.0-rc.5
[0.1.0-rc.4]: https://github.com/kerbaras/mangas/compare/v0.1.0-rc.3...v0.1.0-rc.4
[0.1.0-rc.1]: https://github.com/kerbaras/mangas/releases/tag/v0.1.0-rc.1

