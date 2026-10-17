import type { ImportReport } from "./ipc";

const plural = (n: number, word: string) => `${n} ${word}${n === 1 ? "" : "s"}`;

/** Status line after an import, e.g. `Imported 3 chapters (72 pages) of Berserk · 1 skipped`. */
export function importSummary(r: ImportReport): string {
  const pages = r.imported.reduce((sum, c) => sum + c.pages, 0);
  const parts = [
    r.imported.length === 0
      ? `Nothing new in ${r.manga.title}`
      : `Imported ${plural(r.imported.length, "chapter")} (${plural(pages, "page")}) of ${r.manga.title}`,
  ];
  const unnumbered = r.imported.filter((c) => !c.number).length;
  if (unnumbered) parts.push(`${unnumbered} without a number`);
  if (r.skipped.length) parts.push(`${r.skipped.length} skipped`);
  return parts.join(" · ");
}
