import { sortChapters } from "./chapters";
import type { Chapter, ExportFormat, Manga } from "./ipc";
import { chapterTag, seasonTag } from "./season-tag";

export type ExportPick = { manga: Manga; chapters: Chapter[] };

/**
 * `Title ch 7`, `Title ch 1-5 (3 ch)`, or `Title +2 more` when several manga are picked. The
 * count disambiguates different selections with the same first and last chapter. Season-numbered
 * titles use their tag instead: `Title S2 Ep. 5`, `Title S1 Ep. 0 - S2 Ep. 3 (82 ch)`.
 */
export function exportTitle(picks: ExportPick[]): string {
  const [first, ...rest] = picks.filter((p) => p.chapters.length > 0);
  if (!first) return "Manga";
  if (rest.length > 0) return `${first.manga.title} +${rest.length} more`;
  const sorted = sortChapters(first.chapters);
  const [lo, hi] = [sorted[0], sorted[sorted.length - 1]];
  if (sorted.length === 1) return `${first.manga.title} ${chapterTag(lo)}`;
  const range =
    seasonTag(lo) || seasonTag(hi)
      ? `${chapterTag(lo)} - ${chapterTag(hi)}`
      : `ch ${lo.number}-${hi.number}`;
  return `${first.manga.title} ${range} (${sorted.length} ch)`;
}

/** A filename-safe version of `title` with the extension of `format`. */
export const exportFileName = (title: string, format: ExportFormat = "epub"): string =>
  `${title
    .replace(/[\\/:*?"<>|]+/g, " ")
    .replace(/\s+/g, " ")
    .trim()
    .slice(0, 120)}.${format}`;

export const joinPath = (dir: string, file: string): string =>
  `${dir.replace(/[\\/]+$/, "")}/${file}`;

export const parentDir = (path: string): string => {
  const cut = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  if (cut < 0) return path;
  return cut === 0 ? path[0] : path.slice(0, cut);
};
