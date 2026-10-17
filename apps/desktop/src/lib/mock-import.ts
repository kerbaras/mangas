/** Fake local import for `pnpm dev:web`: three numbered chapters from the picked folder. */
import type { Chapter, ImportReport } from "./ipc";
import { LOCAL_SOURCE } from "./ipc";

const slug = (s: string) =>
  s
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "") || "imported";

export function mockImport(paths: string[], title: string | undefined, dataDir: string) {
  const folder = paths[0]?.split(/[\\/]/).filter(Boolean).pop() ?? "Imported";
  const name = title?.trim() || folder;
  const id = slug(name);
  const chapters: Chapter[] = ["1", "2", "3"].map((number) => ({
    id: `${id}-${number}`,
    manga_id: id,
    title: `Chapter ${number}`,
    language: "en",
    volume: null,
    number,
    local_path: `${dataDir}/${LOCAL_SOURCE}/${id}/${number}`,
  }));
  const report: ImportReport = {
    manga: {
      id,
      source: LOCAL_SOURCE,
      title: name,
      description: "",
      cover_url: null,
      status: "unknown",
    },
    imported: chapters.map((c) => ({
      number: c.number,
      title: c.title,
      pages: 24,
      path: c.local_path ?? "",
    })),
    skipped: [{ name: "notes.txt", reason: "not a chapter folder or .cbz file" }],
  };
  return { report, chapters };
}
