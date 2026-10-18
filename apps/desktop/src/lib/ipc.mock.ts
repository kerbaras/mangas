/**
 * In-browser stand-in for the Tauri shell. Keeps a library in memory, fakes network latency and
 * drives the same progress events the Rust side emits, so every screen works in `pnpm dev:web`.
 */
import type {
  Backend,
  Chapter,
  DownloadProgress,
  ExportProgress,
  Manga,
  Settings,
  Unlisten,
} from "./ipc";
import {
  sampleProviders as providers,
  sampleCatalog,
  sampleChapters,
  sampleDevices,
  sampleLibrary,
} from "./mock-data";
import { mockDiagnostics } from "./mock-diagnostics";
import { mockExportResult, mockFlags } from "./mock-flags";
import { mockImport } from "./mock-import";
import { mockSearch } from "./mock-search";

const DATA_DIR = "~/.local/share/mangas";

let settings: Settings = {
  data_dir: null,
  default_device: null,
  default_preset: null,
  provider_sessions: {},
  preferred_languages: [],
  provider_options: {},
};

const library = new Map(
  (mockFlags.emptyLibrary ? [] : sampleLibrary).map((m) => [`${m.source}:${m.id}`, m]),
);
const chapterCache = new Map<string, Chapter[]>();
const downloadListeners = new Set<(p: DownloadProgress) => void>();
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

async function simulateDownload(chapter: Chapter, fail: boolean) {
  const pages = 18 + (chapter.id.length % 7);
  for (let page = 1; page <= pages; page++) {
    await wait(90 + Math.random() * 120);
    if (fail && page === Math.floor(pages / 2)) {
      for (const cb of downloadListeners)
        cb({ chapter_id: chapter.id, page, pages, done: true, error: "HTTP 429 from source" });
      return;
    }
    const done = page === pages;
    if (done) chapter.local_path = `${DATA_DIR}/mangadex/${chapter.manga_id}/${chapter.number}`;
    for (const cb of downloadListeners)
      cb({ chapter_id: chapter.id, page, pages, done, error: null });
  }
}

const stages = [
  "Reading pages",
  "Detecting cut points",
  "Cutting strips",
  "Filtering for e-ink",
  "Writing EPUB",
];

async function simulateExport(total: number) {
  for (const stage of stages) {
    for (let current = 1; current <= total; current++) {
      await wait(60);
      for (const cb of exportListeners) cb({ stage, current, total });
    }
  }
}

export const mockBackend: Backend = {
  async listLibrary() {
    await wait(250);
    return [...library.values()];
  },
  async listDevices() {
    return sampleDevices;
  },
  async defaultDataDir() {
    return DATA_DIR;
  },
  async getSettings() {
    await wait(50);
    return settings;
  },
  async setSettings(next) {
    settings = { ...next, provider_sessions: settings.provider_sessions };
  },
  async installCli() {
    await wait(200);
    return "/usr/local/bin/mangas";
  },
  async listProviders() {
    return providers;
  },
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
  async followManga(source, id) {
    await wait(200);
    const manga: Manga | undefined = sampleCatalog.find((m) => m.source === source && m.id === id);
    if (!manga) throw new Error(`Unknown manga ${source}:${id}`);
    library.set(`${source}:${id}`, manga);
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
    void (async () => {
      for (const [i, chapter] of picked.entries()) await simulateDownload(chapter, i === 2);
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
    await simulateExport(Math.max(1, request.chapter_dirs.length * 6));
    return mockExportResult(request.output, request.chapter_dirs.length);
  },
  async detectorsStatus() {
    await wait(120);
    return { available: true, downloaded: true, provider: "cpu" };
  },
  ...mockDiagnostics(DATA_DIR, providers, () => settings),
  onDownloadProgress: (cb) => subscribe(downloadListeners, cb),
  onExportProgress: (cb) => subscribe(exportListeners, cb),
};
