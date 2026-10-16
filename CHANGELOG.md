# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
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
[0.1.0-rc.4]: https://github.com/kerbaras/mangas/compare/v0.1.0-rc.3...v0.1.0-rc.4
[0.1.0-rc.1]: https://github.com/kerbaras/mangas/releases/tag/v0.1.0-rc.1

