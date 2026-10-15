# Mangas rewrite: Rust + Tauri + React

## Why

The Go version was built around a terminal UI nobody will use. The product is a desktop app
(library, search, download, export to Kindle) with a thin CLI for scripting. Rust gives one
language for the core, the CLI and the Tauri shell, and first-class ONNX Runtime bindings for
the speech-bubble/face detectors used by smart cutting.

## Layout (cargo workspace + pnpm workspace)

```
crates/core        mangas-core: domain, sources, library (SQLite), download, epub/kindle, smartcut, ml
crates/cli         mangas: clap CLI, one file per subcommand, calls core only
apps/desktop       Tauri 2 shell (src-tauri) + React/Vite/Tailwind UI (src); IPC commands are thin
                   wrappers over core
docs/              design notes
```

Rules: the Tauri layer and the CLI contain no business logic. Everything testable lives in
`mangas-core`, with integration tests under `crates/core/tests/`.

## Core modules

| module | responsibility |
|---|---|
| `domain` | `Manga`, `Chapter`, `Page`, ids, errors (`thiserror`) |
| `sources` | `Source` trait (`search`, `manga`, `chapters`, `pages`); `mangadex` impl (`reqwest`) |
| `library` | SQLite via `sqlx` (offline mode, migrations in `migrations/`); repository functions |
| `download` | chapter download queue, concurrency, resume, CBZ on disk |
| `export::epub` | fixed-layout EPUB3 writer (`zip`), pre-paginated metadata for Kindle |
| `export::kindle` | device profiles, e-ink filters (grayscale, tone LUT, sharpen) |
| `smartcut` | row features, cut costs, DP paginator, strip stitching (port of the Go code) |
| `ml` | ONNX detectors via `ort`: bubbles/text (RT-DETR) and faces; model download + cache |

## Decisions

- **Storage: SQLite (`sqlx`)**, not DuckDB. A desktop library is a few thousand rows; SQLite is
  the standard embedded store and `sqlx` gives compile-time checked queries.
- **UI kit: shadcn/ui on Tailwind** with Fluent 2 spacing and type scale as tokens. Native window
  chrome per OS (Tauri `decorations: true`), no in-app titlebar branding.
- **Models**: `ogkalu/comic-text-and-bubble-detector` (RT-DETR, Apache-2.0, ONNX + int8 variant)
  for bubbles and text, `deepghs/anime_face_detection` (MIT) for faces. The YOLOv8 bubble model
  (`kitsumed`) is GPL-3.0 and is not shipped. Models are downloaded on first use into the app
  data dir (`~/.local/share/mangas/models`), verified by SHA-256, never bundled in the installer.
- **Inference**: `ort` with execution providers chosen at runtime: CUDA (Linux/Windows with NVIDIA),
  DirectML (Windows), CoreML (macOS), CPU fallback. Detection is optional; smart cut works without
  models using the classical features alone.
- **Smart cut defaults**: preset `balanced` (shrink <= 10%, overlap 7.5%), as agreed on the Go PR.

## Coding rules (also in AGENTS.md)

- Files under 200 lines; never over 400. Split by responsibility, one concern per module.
- Prefer pure functions over structs with state; data in, data out. IO at the edges.
- No `unwrap()` outside tests; errors are typed with `thiserror`, surfaced with `anyhow` in binaries.
- `cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`,
  `pnpm check` must pass before pushing.

## Delivery

1. PR 1: workspace scaffold, domain types, `Source` trait, SQLite schema, CLI skeleton, desktop
   shell that boots and shows the library screen with mock data. CI for fmt/clippy/test/typecheck.
2. Stacked PRs (one subdevin each): A sources + download + library; B export + smartcut + ml;
   C desktop UI.
3. Final PR (done): merged A/B/C, UI wired to the real IPC, persisted settings, Go code removed.
