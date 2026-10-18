# AGENTS.md

Mangas is a desktop manga/manhwa library and Kindle exporter: Rust core, Tauri 2 + React desktop
app, small CLI. The original Go/TUI implementation was removed once the Rust port reached parity.

## Layout

- `packages/core` (`mangas-core`): all business logic. Modules: `domain`, `sources` (the
  `Source` trait only), `library`, `download`, `export`, `send` (email/USB delivery), `smartcut`,
  `ml`. Tests in `packages/core/tests/`.
- `packages/providers` (`mangas-providers`): one module per site (`mangadex`, `mangadot`, `weebcentral`, `tapas`), each a
  `Provider` descriptor (`info()` + `build(&Settings)` + `build_at` for mock bases) listed in `registry`, on the shared
  `http::SiteClient` (timeouts, rate limit, retries, `HostPolicy` page headers, status probe).
  Sites behind a browser verification (Comick) need a `Session` (cookies + user agent) stored in
  the app settings; MangaDot only needs the site `Referer` on its API, so its session is optional.
  `SiteClient` turns a challenge page into `Error::Challenge` so the UI can offer "Connect". HTML sites parse with the shared `html` helper (pure `scraper` selectors over a body).
  Mapping functions are pure and tested on JSON/HTML fixtures in `packages/providers/tests/`;
  every provider also passes the shared contract suite (`tests/contract_suite.rs`, fixtures in
  `tests/fixtures/<id>/contract/`), and `mangas providers check <id>` runs the same walk live.
  How to add a provider: `docs/rewrite-design.md`.
- `apps/cli` (`mangas-cli`): clap CLI as a library plus a dev binary, one file per subcommand, thin
  calls into core. The shipped `mangas` executable is the desktop one: with arguments it runs
  this CLI, without them it opens the app.
- `apps/desktop`: Tauri shell in `src-tauri` (IPC commands only, no logic) and React UI in `src`.
- `apps/web`: the static landing page (Astro + Tailwind, desktop tokens and icon), deployed to
  GitHub Pages by `.github/workflows/pages.yml`. Its only script resolves the latest release's
  installer (`src/lib`); every link works without it.
- `docs/rewrite-design.md`: architecture and decisions.

## Commands

- Rust: `cargo build --workspace`, `cargo test --workspace`, `cargo fmt --all -- --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`
- Web: `pnpm install`, `pnpm check` (root: lint + typecheck + web tests + build),
  `pnpm -C apps/desktop dev`, `pnpm -C apps/desktop tauri dev`
- CLI: `cargo run -p mangas-cli -- --help`

## Code rules

- Keep files under 200 lines; never exceed 400. Split into modules by single responsibility.
- Functional style: pure functions, data in / data out, IO at the edges (sources, library, fs).
- No `unwrap()`/`expect()` outside tests. Core errors use `thiserror`; binaries use `anyhow`.
- Tauri commands and CLI subcommands only parse input, call core, and format output.
- Folders open only through the `open_folder` command (`openFolder` in `ipc.folders.ts`): it
  requires a real directory inside the app's folders or an export folder. Never grant the webview
  `opener:allow-open-path`. Every other plugin the UI uses (imports or init scripts) needs a
  permission in `capabilities/default.json`; the `plugin_capability` test checks this.
- UI: shadcn/ui components on Tailwind, Fluent 2 spacing/type scale tokens in `src/styles`.
  Desktop feel: native window chrome, no app name in the content area, sentence-case labels.
- Models and the ONNX Runtime library are downloaded at runtime (see `ml::models`,
  `ml::runtime`; `ort` runs in `load-dynamic` mode), never committed or bundled.
