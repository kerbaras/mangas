import type { Enqueued } from "./ipc.downloads";

const chapters = (n: number) => `${n} ${n === 1 ? "chapter" : "chapters"}`;

/** "Queued 3 chapters, skipped 2 already on disk", or null when nothing was skipped. */
export function enqueueSummary({ queued, on_disk }: Enqueued): string | null {
  if (on_disk.length === 0) return null;
  const skipped = `${chapters(on_disk.length)} already on disk`;
  return queued > 0 ? `Queued ${chapters(queued)}, skipped ${skipped}` : `Skipped ${skipped}`;
}
