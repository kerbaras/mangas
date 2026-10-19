/** Browser stand-in for the storage and backup commands. */
import type { Manga, Unlisten } from "./ipc";
import type { MoveProgress, StorageBackend } from "./ipc.storage";

const wait = (ms: number) => new Promise((r) => setTimeout(r, ms));
const MB = 1_000_000;

type Host = { library: () => Manga[]; dataDir: () => string; setDataDir: (dir: string) => void };

export function mockStorage({ library, dataDir, setDataDir }: Host): StorageBackend {
  const deleted = new Set<string>();
  const caches = { covers: 18.4 * MB, models: 236 * MB };
  const listeners = new Set<(p: MoveProgress) => void>();
  const titles = () =>
    library()
      .filter((m) => m.source !== "local")
      .map((m, i) => ({
        source: m.source,
        manga_id: m.id,
        title: m.title,
        chapters: 4 + ((i * 7) % 23),
        bytes: Math.round((900 - i * 170) * MB),
      }))
      .filter((t) => t.bytes > 0 && !deleted.has(`${t.source}:${t.manga_id}`))
      .sort((a, b) => b.bytes - a.bytes);
  return {
    async storageUsage() {
      await wait(250);
      const list = titles();
      return {
        data_dir: dataDir(),
        chapters: list.reduce((sum, t) => sum + t.bytes, 0),
        models: caches.models,
        covers: caches.covers,
        library: 2.1 * MB,
        logs: 0.6 * MB,
        titles: list,
        partial_move: null,
      };
    },
    async deleteDownloads(source, mangaId) {
      await wait(300);
      const title = titles().find((t) => t.source === source && t.manga_id === mangaId);
      deleted.add(`${source}:${mangaId}`);
      return { chapters: title?.chapters ?? 0, bytes: title?.bytes ?? 0, kept: [] };
    },
    async clearCache(cache) {
      await wait(200);
      const bytes = caches[cache];
      caches[cache] = 0;
      return { files: bytes ? 42 : 0, bytes };
    },
    async moveDataDir(to) {
      const total = titles().reduce((sum, t) => sum + t.bytes, caches.models);
      for (let step = 0; step <= 10; step++) {
        for (const cb of listeners) cb({ copied: (total * step) / 10, total });
        await wait(150);
      }
      setDataDir(to);
      return { data_dir: to, files: 1240, bytes: total, leftovers: [] };
    },
    moveTargetPartial: async () => null,
    removePartialMove: async () => null,
    onMoveProgress: async (cb): Promise<Unlisten> => {
      listeners.add(cb);
      return () => listeners.delete(cb);
    },
    async exportBackup() {
      await wait(200);
      return { titles: library().filter((m) => m.source !== "local").length, skipped_local: 0 };
    },
    async importBackup(_path, preferences = "auto") {
      await wait(400);
      const keep = preferences === "keep" || (preferences === "auto" && library().length > 0);
      return {
        added: 2,
        already: library().length,
        skipped: [
          { title: "Omniscient Reader", reason: "asurascans isn't a provider mangas supports" },
        ],
        settings: !keep,
        kept_settings: keep,
      };
    },
  };
}
