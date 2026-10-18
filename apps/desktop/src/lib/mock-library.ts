/** In-memory categories, reading statuses and chapter counts for the browser mock. */
import type { Chapter, Manga } from "./ipc";
import type {
  Category,
  ChapterCounts,
  LibraryBackend,
  LibraryEntry,
  ReadingStatus,
} from "./ipc.library";
import { mockAutoDownload, mockChapterGroup, mockNewCount } from "./mock-updates";

const DAY = 86_400_000;
const key = (source: string, id: string) => `${source}:${id}`;
const numberKey = (n: string) => (Number.isFinite(Number(n)) ? String(Number(n)) : n);

/** Same rules as `library::count_chapters`: numbers once across languages, exported = read. */
export function countChapters(chapters: Chapter[]): ChapterCounts {
  const numbers = (keep: (c: Chapter) => boolean) =>
    new Set(chapters.filter(keep).map((c) => numberKey(c.number)));
  const exported = numbers((c) => !!c.local_path && !!c.exported_at);
  const available = numbers((c) => !c.availability || c.availability.kind === "available");
  return {
    total: numbers(() => true).size,
    downloaded: numbers((c) => !!c.local_path).size,
    exported: exported.size,
    unread: [...available].filter((n) => !exported.has(n)).length,
    new: 0,
  };
}

export function markExported(lists: Iterable<Chapter[]>, dirs: string[]) {
  const at = new Date().toISOString();
  for (const list of lists)
    for (const c of list) if (c.local_path && dirs.includes(c.local_path)) c.exported_at = at;
}

const seedStatuses: ReadingStatus[] = ["reading", "completed", "reading", "on_hold", "reading"];

export function mockLibrary(
  library: Map<string, Manga>,
  chaptersOf: (source: string, id: string) => Chapter[],
): LibraryBackend {
  let categories: Category[] = [
    { id: 1, name: "Favourites" },
    { id: 2, name: "Weekly" },
  ];
  let nextId = 3;
  const statuses = new Map<string, ReadingStatus>();
  const links = new Map<string, number[]>();
  const added = new Map<string, string>();
  const now = Date.now();
  [...library.keys()].forEach((k, i) => {
    statuses.set(k, seedStatuses[i % seedStatuses.length]);
    links.set(k, i % 3 === 0 ? [1] : i % 3 === 1 ? [2] : []);
    added.set(k, new Date(now - (40 - i) * DAY).toISOString());
  });
  const entry = (m: Manga, i: number): LibraryEntry => {
    const k = key(m.source, m.id);
    const addedAt = added.get(k) ?? new Date(now).toISOString();
    return {
      ...m,
      reading_status: statuses.get(k) ?? "reading",
      categories: links.get(k) ?? [],
      added_at: addedAt,
      updated_at: new Date(Math.max(Date.parse(addedAt), now - ((i * 5) % 17) * DAY)).toISOString(),
      chapters: {
        ...countChapters(chaptersOf(m.source, m.id)),
        new: mockNewCount(m.source, m.id, chaptersOf(m.source, m.id)),
      },
      auto_download: mockAutoDownload(m.source, m.id),
      chapter_group: mockChapterGroup(m.source, m.id),
    };
  };
  const taken = (name: string, except?: number) =>
    categories.some((c) => c.id !== except && c.name.toLowerCase() === name.toLowerCase());
  const clean = (name: string, except?: number) => {
    const n = name.trim().replace(/\s+/g, " ");
    if (!n) throw new Error("a category needs a name");
    if (taken(n, except)) throw new Error(`a category named "${n}" already exists`);
    return n;
  };
  return {
    listEntries: async () => [...library.values()].map(entry),
    listCategories: async () => categories,
    async createCategory(name) {
      const created = { id: nextId++, name: clean(name) };
      categories = [...categories, created];
      return created;
    },
    async renameCategory(id, name) {
      const renamed = { id, name: clean(name, id) };
      categories = categories.map((c) => (c.id === id ? renamed : c));
      return renamed;
    },
    async deleteCategory(id) {
      categories = categories.filter((c) => c.id !== id);
      for (const [k, ids] of links)
        links.set(
          k,
          ids.filter((c) => c !== id),
        );
    },
    async setMangaCategories(source, mangaId, ids) {
      links.set(
        key(source, mangaId),
        [...new Set(ids)].sort((a, b) => a - b),
      );
    },
    async setReadingStatus(source, mangaId, status) {
      statuses.set(key(source, mangaId), status);
    },
  };
}
