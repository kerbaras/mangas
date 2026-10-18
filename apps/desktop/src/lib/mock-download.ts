/** One mock chapter download: walks its pages through the queue events, failing where told. */
import type { Chapter } from "./ipc";
import { emitJob, mockJobState } from "./mock-queue-sim";

export type Failure = { page: number; error: string };

const wait = (ms: number) => new Promise<void>((r) => setTimeout(r, ms));

export async function simulateDownload(
  chapter: Chapter,
  source: string,
  failure: Failure | null,
  dataDir: string,
) {
  const pages = 18 + (chapter.id.length % 7);
  emitJob(chapter, source, { state: "running", attempts: 1, pages_total: pages });
  for (let page = 1; page <= pages; page++) {
    await wait(90 + Math.random() * 120);
    if (mockJobState(chapter.id) !== "running") return;
    if (failure && page === failure.page) {
      emitJob(chapter, source, { state: "failed", pages_done: page - 1, error: failure.error });
      return;
    }
    const done = page === pages;
    if (done) chapter.local_path = `${dataDir}/${source}/${chapter.manga_id}/${chapter.number}`;
    const state = done ? "done" : "running";
    emitJob(chapter, source, { state, pages_done: page, local_path: chapter.local_path });
  }
}
