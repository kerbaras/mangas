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
apps/web           Astro + Tailwind landing page (static, GitHub Pages), reuses the desktop tokens
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
| `export` | `Exporter` trait (EPUB, CBZ, PDF), data-driven e-reader `profiles` |
| `export::kindle` | page pipeline shared by every format, e-ink filters (grayscale, tone LUT, sharpen) |
| `smartcut` | row features, cut costs, DP paginator, strip stitching (port of the Go code) |
| `ml` | ONNX detectors via `ort`: bubbles/text (RT-DETR) and faces; model download + cache |

## Decisions

- **Storage: SQLite (`sqlx`)**, not DuckDB. A desktop library is a few thousand rows; SQLite is
  the standard embedded store and `sqlx` gives compile-time checked queries.
- **UI kit: shadcn/ui on Tailwind** with Fluent 2 spacing and type scale as tokens. Native window
  chrome per OS (Tauri `decorations: true`), no in-app titlebar branding.
- **Models**: `ogkalu/comic-text-and-bubble-detector` (RT-DETR, Apache-2.0, ONNX + int8 variant)
  for bubbles and text, `deepghs/anime_face_detection` (MIT) for faces. The YOLOv8 bubble model
  (`kitsumed`) is GPL-3.0 and is not shipped. Models are downloaded on demand into the data
  folder (`~/.local/share/mangas/models`), verified by SHA-256, never bundled in the installer.
- **ONNX Runtime**: every release is built with `ml` in `ort`'s `load-dynamic` mode, so neither the
  build machine nor the installer carries the runtime. `ml::runtime` pins one official
  microsoft/onnxruntime archive per release target (1.28.3; Intel macOS 1.23.2, the last Intel
  build), downloads it with the models ("Download detector support" in the export dialog or
  Settings → Detectors, `mangas ml download`), checks the archive and library SHA-256, keeps only
  the library in `models/runtime/<version>/` and calls `ort::init_from` once before the first
  session. Living under `models/`, it moves with the data folder, counts as detector support in
  storage usage, is deleted with the models and stays out of backups. A failed download or load
  leaves export working without detectors. A download, a data-folder move and a removal
  (`ml remove`, Settings, Storage) lock `models/.detectors.lease` exclusively (`ml::lease`, an
  OS file lock, so a crashed holder frees it; the owner's pid and purpose go in
  `.detectors.lease.owner` for the refusal message), and loading detectors for an export shares
  it. The app and the CLI therefore never change the folder under one another. A move doesn't
  copy the lock files and clearing the models keeps them.
- **Inference**: `ort` with execution providers chosen at runtime: CoreML (Apple silicon), CUDA
  (opt-in `cuda` feature), CPU everywhere else. Windows runs on CPU: the official runtime archive
  has no DirectML provider. Detection is optional; smart cut works without
  models using the classical features alone.
- **Smart cut defaults**: preset `balanced` (shrink <= 10%, overlap 7.5%), as agreed on the Go PR.

## Provider harness: how to add a provider

`packages/providers` gives every site the same plumbing, so a provider is only its mapping:

- **`Provider`** (`provider.rs`): `info() -> ProviderInfo` (id, name, site, languages,
  capabilities, `needs_browser_session`, icon URL), `build(&Settings) -> Arc<dyn Source>` and
  `build_at(&Settings, base)`, the same source with every request sent to `base` (the contract
  tests point it at a mock server).
  `registry.rs` lists the descriptors; `providers()`, `source(id, settings)` and
  `status(id, settings)` are what the CLI and the Tauri commands call.
- **`SiteClient`** (`http/client.rs`): timeouts (30 s, 10 s connect), a token bucket per host
  shared by every client and call in the process (`rate_limit::HostLimiters`, keyed by
  `host:port`; the strictest declared `requests_per_second` wins),
  retries with exponential backoff (3 retries, 0.5 s doubling, capped at 30 s) that honour
  `Retry-After`. GETs retry on 429/5xx and transport errors; POSTs only on 429 and failed
  connections. A Cloudflare page becomes `Error::Challenge` (never retried) so the UI offers
  Connect. `get`, `get_json::<T>` (errors name the path) and `post_json`.
- **`HostPolicy`** (`http/host_policy.rs`): page headers come from the client. The site and its
  subdomains (https only) get referer + session cookies + user agent; declared `image_hosts` get
  the referer only, over https (`"*"` = every host); anything else gets nothing, so cookies never leak to CDNs.
  Requests the client sends to its own base (`request_headers`) all carry `Referer: <site>/`;
  `/api` paths, the site's own `fetch()` calls, also get `Origin: <site>` and, with a session,
  `sec-fetch-site: same-origin`, `sec-fetch-mode: cors`, `sec-fetch-dest: empty`. Cloudflare site
  rules (MangaDot) challenge API calls that do not look same-origin, even with valid cookies.
  With the Referer, MangaDot's API answers without any cookies, so its session is optional
  (`needs_browser_session: false`): the status probe always runs, and a challenge still becomes
  `Error::Challenge` / `NeedsSession`, so Connect stays the fallback. Its HTML pages stay
  challenged for non-browser clients even with a session, so it reads everything from the API.
- **`Session`**: cookies + user agent + `saved_at` from the settings; `ClientConfig.session_ttl`
  makes older sessions report `NeedsSession` once the host answers the probe (a host that
  can't be reached is `Unreachable`, session or not).
- **Preferences**: `Settings.preferred_languages` (generic, never empty via
  `Settings::languages()`) and `Settings.provider_options[<id>]`, a JSON value each provider
  deserialises into its own typed options with defaults (e.g. `mangadex::MangaDexOptions`:
  content ratings, data-saver, preferred groups). The core never knows their shape.
- **Mirrors**: `Source::mirror_url(url)` names a second URL for a page; downloads retry a failed
  page there once (`download::WithMirror`). MangaDex maps MD@Home nodes to `uploads.mangadex.org`.
- **Page downloads**: core only knows `download::PageFetcher`; the apps pass the providers'
  `http::PageClient`, so pages get the source's page headers, the host's shared token bucket
  (`image_hosts` take their provider's `requests_per_second`, other hosts `DEFAULT_PAGE_RATE`),
  `PAGE_RETRY` with `Retry-After`, and public hosts only. A 429 pauses the whole host for the
  retry delay. "Public" is checked on the addresses a page host resolves to, so a name pointing
  at the LAN or loopback is refused too. Through a proxy (Settings → Network or the `*_PROXY`
  variables) the proxy resolves the host and no local lookup runs (#123): a host only the
  proxy can resolve (split DNS, a VPN) downloads, and the trade-off is that only private IP
  literals and `localhost` in the URL are still refused; what a name points at is the proxy's
  call. The proxy's own name may resolve to a private address (it is the user's choice). A chapter with any page missing after
  retries is an error naming the failed pages and the count; it is never marked downloaded.
- **Download queue** (`download::queue`, #29): every download is a row in `download_jobs`
  (one per chapter), so it survives restarts and the desktop app and CLI share it. The logic is
  pure (`transitions` per job, `schedule::pick` for 2 per provider / 4 overall, `retry` with
  15 s/60 s backoff over 3 attempts, `tick::plan_tick` for one worker step, `events::diff` for
  UI events); `worker::Queue` is the async edge. A running job is leased to one process
  (`owner` + `heartbeat_at`, 15 s); claims run in IMMEDIATE transactions so the caps hold
  across processes, a stale lease goes back to the queue, and a pause or cancel from any process
  stops the task at the next tick. The runner the apps use is `mangas_providers::QueueRunner`.
- **Status**: `SiteClient::status(probe)` does one cheap GET and maps it to `Ready`,
  `NeedsSession` (no or expired session, or a challenge page) or `Unreachable { reason }`.

- **HTML** (`html/`): sites without a JSON API parse with pure functions over `scraper`:
  `parse(body)`, `select`/`first` (CSS selectors; an invalid one is an error, not a panic),
  `text`, `text_lines`, `attr`, `meta` (`og:*` and named tags) and `absolute(base, href)`.
  WeebCentral is the reference: one module per page kind, each `fn(&str) -> Result<_>`.

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
        self.build_at(settings, "https://example.com/api")
    }
    fn build_at(&self, settings: &Settings, base: &str) -> Result<Arc<dyn Source>> {
        let config = ClientConfig { image_hosts: &["cdn.example.com"], ..ClientConfig::new(INFO, base) };
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

### Testing a provider

Every registered provider goes through the same contract suite
(`packages/providers/tests/contract_suite.rs`, helpers in `tests/contract/`). It only uses the
`Provider`/`Source` API, so a new provider needs fixtures, not test code:

1. Record the responses of one walk (search → manga → chapters → pages) with
   `scripts/record-fixture.sh <id> contract/<file> '<url>'`. For a site behind a verification,
   pass the stored session's cookies and user agent with `-c` and `-a`. Trim the bodies: two
   search results, a few chapters. Keep at least one chapter in another language and one out of
   order, so the language filter and the sort get exercised. Paginated bodies must report a
   total that ends the paging.
2. Write `tests/fixtures/<id>/contract/manifest.json`: `query`, `language`,
   `pages_need_referer`, optionally `challenge_without_referer` (a path prefix answering the
   Cloudflare page to requests without a `Referer`, as MangaDot's `/api/` does), and `routes` (the `path` relative to the `build_at` base, the required
   `query` parameters, and the `file` that answers it). The first search result must be the
   manga whose chapters are recorded, and its first chapter after sorting the one whose pages
   are recorded. A provider that talks to several hosts (Webtoons: `www.` pages and the `m.`
   API) points all of them at the one `build_at` base, so their paths must not collide; page
   URLs stay absolute on the real image CDN so the header checks see the `image_hosts` rule.
3. `cargo test -p mangas-providers --test contract_suite` checks:
   - unique ids with titles, and `manga(id)` round-trips
   - chapters of that manga, in the requested language, sorted by `Chapter::number`, numbers
     that parse
   - non-empty absolute page URLs
   - page headers: the referer where the manifest says so, session cookies only on the site's
     own host
   - a Cloudflare page becomes `Error::Challenge`
   - the status probe maps 200, a challenge and 500 to Ready, NeedsSession and Unreachable;
     a missing required session gives NeedsSession when the host answers, Unreachable when
     it doesn't

   A registered provider without a manifest fails the suite.
4. Live: add `<id>_live_contract` / `<id>_live_check` to `tests/live.rs` if CI can reach the
   site. They are `#[ignore]` and do nothing unless `MANGAS_LIVE=1`:
   `MANGAS_LIVE=1 cargo test -p mangas-providers --test live -- --ignored <id>`. The manual
   `live-smoke` workflow runs them (MangaDex, WeebCentral, Tapas and Webtoons; Comick's
   search is Cloudflare-gated from CI, so Comick's live tests walk its open endpoints and check
   the missing-session report instead).
5. By hand: `mangas providers check <id> [--query q] [--lang es]` walks status → search → manga
   → chapters → pages → a one-byte GET of the first page with the provider's headers. It
   prints one line per step with timings and exits non-zero on failure. The walk is
   `doctor::check` in the providers crate. `--lang` (default: the first preferred language)
   drives the search filter as well as the chapter list.
6. Options: a provider with settings implements `Provider::options` (its effective options as a
   JSON object, stored under `Settings.provider_options[id]`). `mangas providers options <id>
   --set key=value` then edits them with no CLI code per provider.

## Desktop export flow

The desktop app has one export flow: the export dialog (`components/export-dialog.tsx`).

- **Entry points.** In Details, "Export" opens the dialog on the selected downloaded chapters,
  or on every downloaded chapter of the series (in the language shown) when nothing is selected.
  The Export rail screen is a thin wrapper around the same dialog: pick downloaded chapters
  from across the library, then "Export". That screen also keeps the export history
  ("Recent exports"), where books written earlier can be sent again.
- **What to write.** Bundle mode: one file per chapter, by volume, every N chapters, or one
  book. One file per chapter is the planner's `per_chapter` grouping: each book is named after
  its chapter with `chapter_tag` (`<series> ch 12`, `<series> S2 Ep. 0`), not numbered. Title
  applies when the choice makes one book. The preview lists the files the backend planned
  (`plan_volumes`). For one series the dialog calls `export_volumes`, so every book gets the
  series cover, a TOC and the metadata. Chapters from several series can only be one book,
  which `export_chapters` writes.
- **How.** Device profile and format (EPUB/CBZ/PDF), stored as the defaults. `export_volumes`
  resolves the format like the CLI: explicit, else the stored default, else the profile's.
  Smart cut (`SmartCutSection`): the cut preset and the detectors switch. Per-series smart-cut
  options go in the same section. The switch is enabled once the build has detector
  support (`detectors_status().available`) and it is downloaded; until then the section offers
  "Download detector support (~N MB)" with progress, Cancel and Retry.
- **Where.** Output folder (default `<data dir>/kindle`), then delivery: save to folder, copy to
  the Kindle over USB, or email to Kindle (EPUB/PDF only, needs the mail settings). Delivery runs
  after the files are written, one book at a time. A copy that finds the file already on the
  device reports "Already on Kindle", and every delivery is recorded in the export history.
  When a later book fails, the lines of the books that did go stay next to the error and the
  history is refreshed. Changing the bundle, title, profile, format, folder or delivery clears
  the last result.
- **File names.** The writer never replaces a file: a taken name gets ` (2)`, ` (3)`…
  (`claim_free_file`). The preview shows the same names before writing: `free_export_names`
  lists the output folder and runs the planned names through `resolve_free_names`, which tries
  the same candidates in the same order as the writer.
- **Names.** Internally this is "export": Tauri `export_chapters`, `export_volumes`, the
  `export-progress` event, and the `export` route. "Kindle" is used only for the Kindle-specific
  parts: the `export::kindle` page pipeline, Send to Kindle, and the `mangas kindle` CLI command.

## App updates

`tauri-plugin-updater` installs in place: AppImage, .deb, .rpm, NSIS and the macOS .app (the
.deb path asks for a password through `pkexec`). The rules live in `mangas_core::app_update`;
the desktop (`commands/app_update.rs`) and `mangas update --check` both use them.

- Channels: **stable** (default) and **release candidates**, picked in Settings → About and
  stored as `app_update_channel`. The rc channel also gets full releases.
- Manifests: tauri-action writes each signed release's `latest.json` (`includeUpdaterJson`).
  The `updater` job (`scripts/publish-updater.sh`) copies it to the rolling `updater`
  pre-release: `latest-rc.json` for every release, `latest.json` for stable ones only, and only
  over an older version. Apps read `releases/download/updater/<file>`, then fall back to
  `releases/latest/download/latest.json` (GitHub's latest release is never a pre-release). So
  stable can never be offered an rc, and re-running an old tag can't move a channel back.
- Versions: an update must be strictly newer (semver, `rc.10` > `rc.2`); a pre-release counts
  only on the rc channel. The same version is "up to date"; an older one is its own outcome
  (`older`, nothing to install). A newer release without a build for this OS/arch is an
  error, and one whose signature carries another key id than `plugins.updater.pubkey` is
  `invalid_signature` (never offered; the plugin still verifies the full signature when it
  downloads).
- Signing: bundles are signed with `TAURI_SIGNING_PRIVATE_KEY` (+ `_PASSWORD`) through
  `src-tauri/tauri.updater.conf.json` (`createUpdaterArtifacts`), only added when the secret is
  set, so local builds and keyless CI still bundle. Without the key the release still publishes,
  with a warning and no manifest. The public key is `plugins.updater.pubkey` in
  `tauri.conf.json`; the publish step refuses a manifest signed by any other key
  (`scripts/test-publish-updater.sh` dry-runs it in CI against a fake `gh`).
- Checks: one silent check per launch (failures only logged), unless "Check for updates at
  launch" is off (`app_update_checks`). Nothing downloads until "Restart to update"; the banner
  then shows progress and the app relaunches. `MANGAS_UPDATE_URL` points every channel at one
  manifest URL, for testing against a local server.

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
