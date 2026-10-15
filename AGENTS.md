# AGENTS.md

Mangas is a desktop manga/manhwa library and Kindle exporter: Rust core, Tauri 2 + React desktop
app, small CLI. The Go code under `cmd/` and `pkg/` is legacy and is being removed; do not extend it.

## Layout

- `crates/core` (`mangas-core`): all business logic. Modules: `domain`, `sources`, `library`,
  `download`, `export`, `smartcut`, `ml`. Tests in `crates/core/tests/`.
- `crates/cli` (`mangas`): clap CLI, one file per subcommand, thin calls into core.
- `apps/desktop`: Tauri shell in `src-tauri` (IPC commands only, no logic) and React UI in `src`.
- `docs/rewrite-design.md`: architecture and decisions.

## Commands

- Rust: `cargo build --workspace`, `cargo test --workspace`, `cargo fmt --all -- --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`
- Web: `pnpm install`, `pnpm -C apps/desktop dev`, `pnpm -C apps/desktop check` (tsc + eslint),
  `pnpm -C apps/desktop tauri dev`
- CLI: `cargo run -p mangas -- --help`

## Code rules

- Keep files under 200 lines; never exceed 400. Split into modules by single responsibility.
- Functional style: pure functions, data in / data out, IO at the edges (sources, library, fs).
- No `unwrap()`/`expect()` outside tests. Core errors use `thiserror`; binaries use `anyhow`.
- Tauri commands and CLI subcommands only parse input, call core, and format output.
- UI: shadcn/ui components on Tailwind, Fluent 2 spacing/type scale tokens in `src/styles`.
  Desktop feel: native window chrome, no app name in the content area, sentence-case labels.
- Models are downloaded at runtime (see `ml::models`), never committed or bundled.
