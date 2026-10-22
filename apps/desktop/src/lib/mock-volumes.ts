import { withExtension } from "./export-plan";
import type { Chapter, ExportProgress, Manga } from "./ipc";
import type { BookBudget, EstimateRequest, VolumesBackend } from "./ipc.volumes";
import { bookCreators } from "./manga-meta";
import { simulateExport } from "./mock-export";
import { mockExportResult } from "./mock-flags";
import { markExported } from "./mock-library";
import { mockPlan, portable } from "./volume-plan";

type Deps = {
  manga: (source: string, id: string) => Manga | undefined;
  chapters: (source: string, id: string) => Chapter[];
  cached: () => Iterable<Chapter[]>;
  listeners: Set<(p: ExportProgress) => void>;
  /** The default export folder, for a request without one. */
  exportDir: () => string;
};

export function mockVolumes(deps: Deps): VolumesBackend {
  const plan: VolumesBackend["planVolumes"] = async (r) => {
    const picked = deps.chapters(r.source, r.mangaId).filter((c) => r.chapterIds.includes(c.id));
    const manga = deps.manga(r.source, r.mangaId);
    const authors = manga ? bookCreators(manga) : [];
    return mockPlan(manga?.title ?? "Manga", picked, r.grouping, r.title, authors);
  };
  return {
    planVolumes: plan,
    async exportVolumes(r) {
      const books = await plan(r);
      const results = [];
      for (const [index, book] of books.entries()) {
        for (const cb of deps.listeners)
          cb({
            stage: `Book ${index + 1} of ${books.length}: ${book.title}`,
            current: 0,
            total: null,
          });
        await simulateExport(deps.listeners, Math.max(1, book.chapters.length * 2));
        const dirs = book.chapters.flatMap((c) => (c.local_path ? [c.local_path] : []));
        markExported(deps.cached(), dirs);
        results.push(
          mockExportResult(
            `${r.outputDir ?? deps.exportDir()}/${withExtension(book.file_name, r.format)}`,
            book.chapters.length,
          ),
        );
      }
      return results;
    },
    freeExportNames: async (_dir, names) => names,
    checkExportDir: async () => {},
    exportFileName: async (title, format) => withExtension(portable(title), format),
    // About 24 pages a chapter at ~6 kB per quality point: 85 makes a 12 MB chapter.
    estimateExport: async (r) => r.books.map((b) => mockBudget(b.length, r)),
    fitLevers: async (r) => r.books.map((b) => mockBudget(b.length, r, true).levers ?? noLevers),
  };
}

/** A stand-in for `kindle::plan_budget`: 24 pages a chapter, bytes growing with the quality. */
function mockBudget(chapters: number, r: EstimateRequest, always = false): BookBudget {
  const [pages, quality] = [chapters * 24, r.quality ?? 90];
  const bytesAt = (q: number) => pages * q * 6_000;
  const budget = r.email ? 50_000_000 : 200_000_000;
  const [target, split] = [(budget / 10) * 9, r.splitMb ? r.splitMb * 1_000_000 : Infinity];
  const bytes = bytesAt(quality);
  const over = Math.min(bytes, split) > budget;
  const fewer = Math.max(1, Math.min(chapters - 1, Math.floor((chapters * target) / bytes)));
  const levers = {
    split_bytes: target,
    chapters_per_book: chapters > 1 ? fewer : null,
    jpeg_quality: [80, 70, 60, 50].find((q) => q < quality && bytesAt(q) <= target) ?? null,
  };
  const files = Number.isFinite(split) ? Math.max(1, Math.ceil(bytes / split)) : 1;
  const fields = { pages, estimated_bytes: bytes, budget_bytes: budget, files };
  return {
    ...fields,
    over_budget: over,
    jpeg_quality: quality,
    levers: over || always ? levers : null,
  };
}

const noLevers = { split_bytes: 0, chapters_per_book: null, jpeg_quality: null };
