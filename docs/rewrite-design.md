# Mangas rewrite: Rust + Tauri + React

## Why

The Go version was built around a terminal UI nobody will use. The product is a desktop app
(library, search, download, export to Kindle) with a thin CLI for scripting. Rust gives one
language for the core, the CLI and the Tauri shell, and first-class ONNX Runtime bindings for
the speech-bubble/face detectors used by smart cutting.

## Layout (cargo workspace + pnpm workspace)

```
packages/core        mangas-core: domain, sources, library (SQLite), download, epub/kindle, smartcut, ml
apps/cli         mangas: clap CLI, one file per subcommand, calls core only
apps/desktop       Tauri 2 shell (src-tauri) + React/Vite/Tailwind UI (src); IPC commands are thin
                   wrappers over core
docs/              design notes
```

Rules: the Tauri layer and the CLI contain no business logic. Everything testable lives in
`mangas-core`, with integration tests under `packages/core/tests/`.

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

## Provider harness: how to add a provider

`packages/providers` gives every site the same plumbing, so a provider is only its mapping:

- **`Provider`** (`provider.rs`): `info() -> ProviderInfo` (id, name, site, languages,
  capabilities, `needs_browser_session`, icon URL) and `build(&Settings) -> Arc<dyn Source>`.
  `registry.rs` lists the descriptors; `providers()`, `source(id, settings)` and
  `status(id, settings)` are what the CLI and the Tauri commands call.
- **`SiteClient`** (`http/client.rs`): timeouts (30 s, 10 s connect), per-client token bucket,
  retries with exponential backoff (3 retries, 0.5 s doubling, capped at 30 s) that honour
  `Retry-After`. GETs retry on 429/5xx and transport errors; POSTs only on 429 and failed
  connections. A Cloudflare page becomes `Error::Challenge` (never retried) so the UI offers
  Connect. `get`, `get_json::<T>` (errors name the path) and `post_json`.
- **`HostPolicy`** (`http/host_policy.rs`): page headers come from the client. The site and its
  subdomains (https only) get referer + session cookies + user agent; declared `image_hosts` get
  the referer only, over https (`"*"` = every host); anything else gets nothing, so cookies never leak to CDNs.
- **`Session`**: cookies + user agent + `saved_at` from the settings; `ClientConfig.session_ttl`
  makes older sessions report `NeedsSession` without a request.
- **Status**: `SiteClient::status(probe)` does one cheap GET and maps it to `Ready`,
  `NeedsSession` (no or expired session, or a challenge page) or `Unreachable { reason }`.

Recipe (`packages/providers/src/<site>/mod.rs`, mapping in `mapping.rs` tested on fixtures):

```rust
pub const INFO: ProviderInfo = ProviderInfo {
    id: "example", name: "Example", site: "https://example.com", languages: &["en"],
    capabilities: &[Capability::Search, Capability::Chapters, Capability::Download],
    needs_browser_session: false, icon_url: None,
};
pub struct ExampleProvider;
impl Provider for ExampleProvider {
    fn info(&self) -> ProviderInfo { INFO }
    fn build(&self, settings: &Settings) -> Result<Arc<dyn Source>> {
        let config = ClientConfig { image_hosts: &["cdn.example.com"], ..ClientConfig::new(INFO, "https://example.com/api") };
        Ok(Arc::new(Example { client: SiteClient::new(config, Session::from_settings(settings, INFO.id))? }))
    }
}
pub struct Example { client: SiteClient }
#[async_trait]
impl Source for Example {
    fn name(&self) -> &'static str { INFO.id }
    async fn search(&self, q: &str, limit: usize) -> Result<Vec<Manga>> {
        let params = [("q", q.to_string()), ("limit", limit.to_string())];
        Ok(mapping::search(self.client.get_json("/search", &params).await?))
    }
    // chapters / pages via the client and pure mapping functions
    fn page_headers(&self, url: &str) -> Vec<Header> { self.client.page_headers(url) }
    async fn status(&self) -> ProviderStatus { self.client.status("/ping").await }
}
```

Then add `&example::ExampleProvider` to `PROVIDERS` in `registry.rs`.

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
