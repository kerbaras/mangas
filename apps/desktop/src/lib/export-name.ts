import { sortChapters } from "./chapters";
import type { Chapter, Manga } from "./ipc";

export type ExportPick = { manga: Manga; chapters: Chapter[] };

/** `Title ch 1-5`, `Title ch 7`, or `Title +2 more` when several manga are picked. */
export function exportTitle(picks: ExportPick[]): string {
  const [first, ...rest] = picks.filter((p) => p.chapters.length > 0);
  if (!first) return "Manga";
  if (rest.length > 0) return `${first.manga.title} +${rest.length} more`;
  const numbers = sortChapters(first.chapters).map((c) => c.number);
  const range = numbers.length === 1 ? numbers[0] : `${numbers[0]}-${numbers[numbers.length - 1]}`;
  return `${first.manga.title} ch ${range}`;
}

/** A filename-safe version of `title` with the `.epub` extension. */
export const exportFileName = (title: string): string =>
  `${title
    .replace(/[\\/:*?"<>|]+/g, " ")
    .replace(/\s+/g, " ")
    .trim()
    .slice(0, 120)}.epub`;

export const joinPath = (dir: string, file: string): string =>
  `${dir.replace(/[\\/]+$/, "")}/${file}`;

export const parentDir = (path: string): string => {
  const cut = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  return cut > 0 ? path.slice(0, cut) : path;
};
