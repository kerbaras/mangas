import { tr } from "@/i18n/tr";
import type { Enqueued } from "./ipc.downloads";

const chapters = (count: number) => tr("details.chapters", { count });

/** "Queued 3 chapters, skipped 2 already on disk", or null when nothing was skipped. */
export function enqueueSummary({ queued, on_disk }: Enqueued): string | null {
  if (on_disk.length === 0) return null;
  const skipped = tr("details.alreadyOnDisk", { chapters: chapters(on_disk.length) });
  return queued > 0
    ? tr("details.queuedSkipped", { queued: chapters(queued), skipped })
    : tr("details.skipped", { skipped });
}
