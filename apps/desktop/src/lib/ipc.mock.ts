/**
 * In-browser stand-in for the Tauri shell. Keeps a library in memory, fakes network latency and
 * drives the same progress events the Rust side emits, so every screen works in `pnpm dev:web`.
 */
import type { Backend, Chapter, ExportProgress, Manga, Settings, Unlisten } from "./ipc";
import {
  sampleProviders as providers,
  sampleCatalog,
  sampleChapters,
  sampleDevices,
  sampleLibrary,
} from "./mock-data";
import { mockDiagnostics } from "./mock-diagnostics";
import { simulateExport } from "./mock-export";
import { mockExportResult, mockFlags } from "./mock-flags";
import { mockImport } from "./mock-import";
import { markExported, mockLibrary } from "./mock-library";
import { emitJob, mockJobState, mockQueue } from "./mock-queue";
import { mockSearch } from "./mock-search";
import { mockStorage } from "./mock-storage";
import { mockUpdates } from "./mock-updates";

const DATA_DIR = "~/.local/share/mangas";
let settings: Settings = {
  data_dir: null,
  default_device: null,
  default_preset: null,
  provider_sessions: {},
  preferred_languages: [],
  provider_options: {},
};

const failSettingsUntil = mockFlags.settingsFail ? Date.now() + 1500 : 0;
const library = new Map(
  (mockFlags.emptyLibrary ? [] : sampleLibrary).map((m) => [`${m.source}:${m.id}`, m]),
);
const chapterCache = new Map<string, Chapter[]>();
const exportListeners = new Set<(p: ExportProgress) => void>();

const wait = (ms: number) => new Promise<void>((r) => setTimeout(r, ms));

function subscribe<T>(set: Set<(p: T) => void>, cb: (p: T) => void): Promise<Unlisten> {
  set.add(cb);
  return Promise.resolve(() => set.delete(cb));
}

function chaptersOf(source: string, id: string): Chapter[] {
  const key = `${source}:${id}`;
  const cached = chapterCache.get(key);
  if (cached) return cached;
  const manga = sampleCatalog.find((m) => m.source === source && m.id === id);
  const list = manga ? sampleChapters(manga, DATA_DIR) : [];
  chapterCache.set(key, list);
  return list;
}

type Failure = { page: number; error: string };

async function simulateDownload(chapter: Chapter, source: string, failure: Failure | null) {
  const pages = 18 + (chapter.id.length % 7);
  emitJob(chapter, source, { state: "running", attempts: 1, pages_total: pages });
  for (let page = 1; page <= pages; page++) {
    await wait(90 + Math.random() * 120);
    if (mockJobState(chapter.id) !== "running") return;
    if (failure && page === failure.page) {
      emitJob(chapter, source, { state: "failed", pages_done: page - 1, error: failure.error });
      return;
    }
    const done = page === pages;
    if (done) chapter.local_path = `${DATA_DIR}/${source}/${chapter.manga_id}/${chapter.number}`;
    const state = done ? "done" : "running";
    emitJob(chapter, source, { state, pages_done: page, local_path: chapter.local_path });
  }
}

/** The core's challenge error for a provider behind a browser check that has no session yet. */
function wallOf(source: string): Failure | null {
  const info = providers.find((p) => p.id === source);
  if (!info?.needs_browser_session || settings.provider_sessions[source]) return null;
  return { page: 1, error: `challenge: ${info.name} needs a browser session for ${info.site}` };
}

export const mockBackend: Backend = {
  async listLibrary() {
    await wait(250);
    return [...library.values()];
  },
  listDevices: async () => sampleDevices,
  async defaultDataDir() {
    return DATA_DIR;
  },
  async getSettings() {
    await wait(50);
    if (Date.now() < failSettingsUntil) throw new Error("library: database is locked");
    return settings;
  },
  async setSettings(next) {
    settings = { ...next, provider_sessions: settings.provider_sessions };
  },
  async installCli() {
    await wait(200);
    return "/usr/local/bin/mangas";
  },
  listProviders: async () => providers,
  async connectProvider(id) {
    await wait(800);
    settings = {
      ...settings,
      provider_sessions: {
        ...settings.provider_sessions,
        [id]: {
          cookies: "cf_clearance=mock",
          user_agent: "mock",
          saved_at: new Date().toISOString(),
        },
      },
    };
  },
  async disconnectProvider(id) {
    const { [id]: _, ...rest } = settings.provider_sessions;
    settings = { ...settings, provider_sessions: rest };
  },
  async providerStatus(id) {
    await wait(300);
    const info = providers.find((p) => p.id === id);
    if (!info) return { state: "unreachable", reason: `not found: source '${id}'` };
    if (info.needs_browser_session && !settings.provider_sessions[id])
      return { state: "needs_session" };
    return { state: "ready" };
  },
  async searchAll(query, only) {
    await wait(300 + Math.random() * 600);
    return mockSearch(query, only, settings);
  },
  coverSrc: async (manga) => manga.cover_url,
  async followManga(source, id) {
    await wait(200);
    const manga: Manga | undefined = sampleCatalog.find((m) => m.source === source && m.id === id);
    if (!manga) throw new Error(`Unknown manga ${source}:${id}`);
    library.set(`${source}:${id}`, manga);
    return manga;
  },
  async getManga(source, id) {
    await wait(300);
    const manga =
      library.get(`${source}:${id}`) ??
      sampleCatalog.find((m) => m.source === source && m.id === id);
    if (!manga) throw new Error(`not found: manga ${source}:${id}`);
    return manga;
  },
  async unfollowManga(source, id) {
    await wait(150);
    library.delete(`${source}:${id}`);
  },
  async listChapters(source, id, refresh = false) {
    await wait(refresh ? 600 : 300);
    return chaptersOf(source, id).map((c) => ({ ...c }));
  },
  async downloadChapters(source, mangaId, chapterIds) {
    const manga = sampleCatalog.find((m) => m.source === source && m.id === mangaId);
    if (manga) library.set(`${source}:${mangaId}`, manga);
    const all = chaptersOf(source, mangaId);
    const picked = chapterIds.flatMap((id) => all.filter((c) => c.id === id));
    for (const c of picked)
      emitJob(c, source, { state: "queued", manga_title: manga?.title ?? "" });
    void (async () => {
      const wall = wallOf(source);
      for (const [i, chapter] of picked.entries())
        await simulateDownload(
          chapter,
          source,
          wall ?? (i === 2 ? { page: 9, error: "HTTP 429 from source" } : null),
        );
    })();
    return [];
  },
  async importLocal(paths, title) {
    await wait(700);
    const { report, chapters } = mockImport(paths, title, DATA_DIR);
    library.set(`${report.manga.source}:${report.manga.id}`, report.manga);
    chapterCache.set(`${report.manga.source}:${report.manga.id}`, chapters);
    return report;
  },
  async exportKindle(request) {
    await simulateExport(exportListeners, Math.max(1, request.chapter_dirs.length * 6));
    markExported(chapterCache.values(), request.chapter_dirs);
    return mockExportResult(request.output, request.chapter_dirs.length);
  },
  async detectorsStatus() {
    await wait(120);
    return { available: true, downloaded: true, provider: "cpu" };
  },
  ...mockDiagnostics(DATA_DIR, providers, () => settings),
  ...mockStorage({
    library: () => [...library.values()],
    dataDir: () => settings.data_dir ?? DATA_DIR,
    setDataDir: (dir) => {
      settings = { ...settings, data_dir: dir };
    },
  }),
  ...mockLibrary(library, chaptersOf),
  ...mockQueue,
  ...mockUpdates(library, chaptersOf, () => settings.update_notifications !== false),
  onExportProgress: (cb) => subscribe(exportListeners, cb),
};
