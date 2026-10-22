import { tr } from "@/i18n/tr";
import type { ImportReport } from "./ipc";

/** Status line after an import, e.g. `Imported 3 chapters (72 pages) of Berserk · 1 skipped`. */
export function importSummary(r: ImportReport): string {
  const pages = r.imported.reduce((sum, c) => sum + c.pages, 0);
  const parts = [
    r.imported.length === 0
      ? tr("import.nothingNew", { title: r.manga.title })
      : tr("import.imported", {
          chapters: tr("import.chapters", { count: r.imported.length }),
          pages: tr("import.pages", { count: pages }),
          title: r.manga.title,
        }),
  ];
  const unnumbered = r.imported.filter((c) => !c.number).length;
  if (unnumbered) parts.push(tr("import.unnumbered", { count: unnumbered }));
  if (r.skipped.length) parts.push(tr("import.skippedCount", { count: r.skipped.length }));
  return parts.join(" · ");
}
