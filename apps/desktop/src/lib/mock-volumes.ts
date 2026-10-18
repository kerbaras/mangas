import type { Chapter, ExportProgress, Manga } from "./ipc";
import type { VolumesBackend } from "./ipc.volumes";
import { simulateExport } from "./mock-export";
import { mockExportResult } from "./mock-flags";
import { markExported } from "./mock-library";
import { mockPlan } from "./volume-plan";

type Deps = {
  manga: (source: string, id: string) => Manga | undefined;
  chapters: (source: string, id: string) => Chapter[];
  cached: () => Iterable<Chapter[]>;
  listeners: Set<(p: ExportProgress) => void>;
};

export function mockVolumes(deps: Deps): VolumesBackend {
  const plan: VolumesBackend["planVolumes"] = async (r) => {
    const picked = deps.chapters(r.source, r.mangaId).filter((c) => r.chapterIds.includes(c.id));
    const title = deps.manga(r.source, r.mangaId)?.title ?? "Manga";
    return mockPlan(title, picked, r.grouping, r.title);
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
        results.push(mockExportResult(`${r.outputDir}/${book.file_name}`, book.chapters.length));
      }
      return results;
    },
  };
}
