# Mangas

Desktop manga/manhwa library and Kindle exporter. Follow series from MangaDex, download chapters,
and turn them into fixed-layout EPUBs sized for a Kindle. Long webtoon strips are cut into pages at
gutters instead of at fixed heights, so panels, speech bubbles and faces stay whole.

Rust core, Tauri 2 + React desktop app, and a small CLI that exposes the same operations.

## Layout

| Path | Crate / package | What it does |
|---|---|---|
| `packages/core` | `mangas-core` | the `Source` contract, SQLite library, downloads, smart cut, Kindle filters, EPUB writer, optional ONNX detectors |
| `packages/providers` | `mangas-providers` | site adapters (MangaDex, MangaDot, WeebCentral, Tapas) on a shared rate-limited client, provider registry, browser-session support for Cloudflare-protected sites |
| `apps/cli` | `mangas-cli` | the command line as a library (+ a dev binary): `search`, `add`, `remove`, `chapters`, `download`, `list`, `kindle`, `export` |
| `apps/desktop` | `@mangas/desktop` | Tauri shell (`src-tauri`, IPC only) and the React UI (`src`); builds the shipped `mangas` executable, which runs the CLI when given arguments and opens the app otherwise |

Design notes and decisions: [`docs/rewrite-design.md`](docs/rewrite-design.md). Rules for
contributors and agents: [`AGENTS.md`](AGENTS.md).

## Install

Builds for macOS (arm64, Intel), Windows (x64, arm64) and Linux (x64, arm64) are on the
[releases page](https://github.com/kerbaras/mangas/releases). Release candidates are marked
pre-release.

The macOS bundle is ad-hoc signed unless the repository has an Apple Developer ID: on first
launch right-click the app and choose Open (or allow it under System Settings → Privacy &
Security). To ship signed and notarized builds, add these repository secrets and the release
workflow picks them up: `APPLE_CERTIFICATE` (base64 `.p12`), `APPLE_CERTIFICATE_PASSWORD`,
`APPLE_SIGNING_IDENTITY`, `APPLE_ID`, `APPLE_PASSWORD` (app-specific password), `APPLE_TEAM_ID`.

## Build

Requirements: Rust 1.85+, Node 24, pnpm 12. The desktop app also needs the
[Tauri 2 system packages](https://v2.tauri.app/start/prerequisites/) (on Ubuntu:
`pkgconf libgtk-3-dev librsvg2-dev libayatana-appindicator3-dev libwebkit2gtk-4.1-dev`).

```bash
cargo build --workspace            # core + CLI (+ desktop backend when the system libs are present)
pnpm install
pnpm -C apps/desktop dev           # desktop app with hot reload
pnpm -C apps/desktop dev:web       # UI alone in a browser, against an in-memory mock backend
```

## CLI

```bash
mangas search "solo leveling"
mangas add <manga-id>
mangas chapters <manga-id>
mangas download <manga-id> --chapters 1-5
mangas settings languages es-la,en  # preferred chapter languages, shared with the app
mangas providers options mangadex --set data_saver=true --set preferred_groups="A,B"
mangas kindle ~/.local/share/mangas/chapters/<manga>/<chapter>... -o out.epub \
    --profile kindle-paperwhite12 --cut-preset balanced
mangas kindle <chapter>... --format cbz --profile kobo-clara   # or pdf; -o book.pdf also picks pdf
mangas kindle --list-profiles
mangas export <manga-id> --by-volume --format cbz --profile kobo-clara   # one book per volume
```

`add`, `chapters` and `download` default `--lang` to the preferred languages (English when none
are saved) and sync every one of them; `download` takes each chapter in the first preferred
language that has it. `--lang es,en` overrides the list for one command.

Data lives in the platform data directory (`~/.local/share/mangas` on Linux): `mangas.db`,
`chapters/`, `models/`.

## Smart cut

A long strip is split with a row-level cut cost (gutters, panel edges, text blocks, bubbles) and a
dynamic-programming paginator that picks the cheapest cut rows for the target page height. When a
cut has to go through art it can repeat a few rows on the next page (overlap) or shrink the page a
little so a whole panel fits. Presets:

| preset | max shrink | behaviour |
|---|---|---|
| `balanced` (default) | 10% | few shrunk pages, cuts at gutters |
| `fewest-cuts` | 25% | fewer cuts through art, smaller text |
| `no-shrink` | 0% | native size, overlap only |

`--detectors` adds ONNX speech-bubble/text and anime-face detectors to the cut cost. Models are
downloaded on first use into `models/` and verified by SHA-256; execution uses CUDA, DirectML or
CoreML when available and falls back to CPU. Builds need the `ml` cargo feature.

## Checks

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
pnpm check      # biome + tsc + vitest + vite build
```
