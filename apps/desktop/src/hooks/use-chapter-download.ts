import { toast } from "sonner";
import { chapterFailed } from "@/hooks/use-downloads";
import { rememberDownloads } from "@/lib/download-labels";
import { enqueueSummary } from "@/lib/enqueue-summary";
import { type Chapter, downloadChapters } from "@/lib/ipc";
import { notifyError } from "@/lib/notify";

type Target = { source: string; mangaId: string; title: string };

/**
 * Queues chapters of one title. Chapters on disk are skipped and the toast says so, with
 * "Download again" to queue those anyway (`force`).
 */
export function useChapterDownload({ source, mangaId, title }: Target, onQueued: () => void) {
  const download = async (picked: Chapter[], force = false) => {
    rememberDownloads(
      picked.map((c) => [c.id, { label: `Chapter ${c.number} of ${title}`, source, mangaId }]),
    );
    try {
      const report = await downloadChapters(
        source,
        mangaId,
        picked.map((c) => c.id),
        force,
      );
      // Queued chapters report through the queue; these ids were not queued at all.
      for (const gone of report.missing)
        chapterFailed(gone, "not found: the source no longer lists this chapter");
      const skipped = picked.filter((c) => report.on_disk.includes(c.id));
      const summary = enqueueSummary(report);
      if (summary)
        toast(summary, {
          id: `skipped-${source}-${mangaId}`,
          action: { label: "Download again", onClick: () => void download(skipped, true) },
        });
      onQueued();
    } catch (e: unknown) {
      notifyError("Download failed", e, {
        source,
        retry: () => void download(picked, force),
        id: `download-${source}`,
      });
    }
  };
  return download;
}
