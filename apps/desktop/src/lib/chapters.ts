import type { Chapter } from "./ipc";

/** Natural numeric order, so "2" sorts before "10" and "12.5" lands between 12 and 13. */
const num = (s: string | null) => {
  const n = Number.parseFloat(s ?? "");
  return Number.isNaN(n) ? Number.POSITIVE_INFINITY : n;
};

export type ChapterSort = "asc" | "desc";

export function sortChapters(chapters: Chapter[], order: ChapterSort = "asc"): Chapter[] {
  const sign = order === "asc" ? 1 : -1;
  return [...chapters].sort(
    (a, b) =>
      sign * (num(a.volume) - num(b.volume) || num(a.number) - num(b.number)) ||
      a.language.localeCompare(b.language),
  );
}

export const languagesOf = (chapters: Chapter[]): string[] =>
  [...new Set(chapters.map((c) => c.language))].sort();

export const filterByLanguage = (chapters: Chapter[], language: string | null): Chapter[] =>
  language ? chapters.filter((c) => c.language === language) : chapters;

export function chapterLabel(c: Chapter): string {
  const parts = [c.volume ? `Vol. ${c.volume}` : null, `Ch. ${c.number}`, c.title];
  return parts.filter(Boolean).join(" · ");
}

export type Selection = ReadonlySet<string>;

export const toggle = (selected: Selection, id: string): Set<string> => {
  const next = new Set(selected);
  if (next.has(id)) next.delete(id);
  else next.add(id);
  return next;
};

/** Shift-click: select everything between the anchor and the target in the visible order. */
export function selectRange(
  selected: Selection,
  visibleIds: string[],
  anchor: string | null,
  target: string,
): Set<string> {
  const from = anchor ? visibleIds.indexOf(anchor) : -1;
  const to = visibleIds.indexOf(target);
  if (from < 0 || to < 0) return toggle(selected, target);
  const [lo, hi] = from < to ? [from, to] : [to, from];
  return new Set([...selected, ...visibleIds.slice(lo, hi + 1)]);
}

/** Header checkbox: clear when everything visible is selected, otherwise select all visible. */
export function toggleAll(selected: Selection, visibleIds: string[]): Set<string> {
  const allOn = visibleIds.length > 0 && visibleIds.every((id) => selected.has(id));
  const next = new Set(selected);
  for (const id of visibleIds) {
    if (allOn) next.delete(id);
    else next.add(id);
  }
  return next;
}

export const isDownloaded = (c: Chapter) => c.local_path !== null;

/** Locked or externally hosted and not on disk: listed, but nothing to download or export. */
export const isUnavailable = (c: Chapter) =>
  !isDownloaded(c) && (c.availability?.kind ?? "available") !== "available";

/** Rows that can be selected; select all and shift-click ranges skip the rest. */
export const selectableIds = (chapters: Chapter[]) =>
  chapters.filter((c) => !isUnavailable(c)).map((c) => c.id);

/** "3 locked, 1 on another site", or null when every chapter can be downloaded. */
export function unavailableSummary(chapters: Chapter[]): string | null {
  const off = chapters.filter(isUnavailable);
  const locked = off.filter((c) => c.availability?.kind === "locked").length;
  const parts = [
    locked > 0 ? `${locked} locked` : null,
    off.length > locked ? `${off.length - locked} on another site` : null,
  ];
  return off.length > 0 ? parts.filter(Boolean).join(", ") : null;
}

export const pickSelected = (chapters: Chapter[], selected: Selection) =>
  chapters.filter((c) => selected.has(c.id) && !isUnavailable(c));

/** What "Download selected" acts on: selected chapters that are not on disk yet. */
export const downloadable = (chapters: Chapter[], selected: Selection) =>
  pickSelected(chapters, selected).filter((c) => !isDownloaded(c));

/** What "Export" acts on: selected chapters that are already on disk. */
export const exportable = (chapters: Chapter[], selected: Selection) =>
  pickSelected(chapters, selected).filter(isDownloaded);

/** What Details' "Export" opens on: the selected downloaded chapters, else every downloaded one. */
export function exportTargets(chapters: Chapter[], selected: Selection): Chapter[] {
  const picked = exportable(chapters, selected);
  return picked.length > 0 ? picked : chapters.filter((c) => !isUnavailable(c) && isDownloaded(c));
}

/** `shown` (default: all of `chapters`) are the rows on screen, which the skipped count covers. */
export function selectionSummary(
  chapters: Chapter[],
  selected: Selection,
  shown: Chapter[] = chapters,
): string {
  const skipped = unavailableSummary(shown);
  const tail = skipped ? ` · ${skipped} skipped` : "";
  const n = pickSelected(chapters, selected).length;
  if (n === 0) return `No chapters selected${tail}`;
  const toGet = downloadable(chapters, selected).length;
  const base = `${n} selected`;
  return (toGet === 0 ? `${base}, all downloaded` : `${base}, ${toGet} to download`) + tail;
}

/** "24 chapters · 5 on disk · 2 exported" under a title on its details screen. */
export function chapterSummary(all: Chapter[]) {
  const onDisk = all.filter((c) => c.local_path).length;
  const exported = all.filter((c) => c.local_path && c.exported_at).length;
  return `${all.length} chapters · ${onDisk} on disk${exported ? ` · ${exported} exported` : ""}`;
}
