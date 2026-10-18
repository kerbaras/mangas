/**
 * Browser-mock download queue: keeps one job per chapter and emits the same `QueueEvent`s the
 * Rust worker does. Pause/resume/cancel/retry only change the state here; the Downloads
 * screen (#30) can grow this further.
 */
import type { Chapter, Unlisten } from "./ipc";
import type { DownloadJob, DownloadsBackend, JobState, QueueEvent } from "./ipc.downloads";

const listeners = new Set<(e: QueueEvent) => void>();
const jobs = new Map<string, DownloadJob>();
let nextId = 1;

const kindOf = (state: JobState, before: JobState | undefined) =>
  state === "running" ? (before === "running" ? "progress" : "started") : state;

/** The state of `chapter`'s job, so the simulated download stops once paused or cancelled. */
export const mockJobState = (chapterId: string) => jobs.get(chapterId)?.state;

/** Creates or updates the job of `chapter` and tells the listeners. */
export function emitJob(chapter: Chapter, source: string, patch: Partial<DownloadJob>) {
  const before = jobs.get(chapter.id);
  const now = Date.now();
  const job: DownloadJob = {
    id: before?.id ?? nextId++,
    source,
    manga_id: chapter.manga_id,
    chapter_id: chapter.id,
    state: "queued",
    attempts: 0,
    error: null,
    pages_done: 0,
    pages_total: 0,
    not_before: 0,
    owner: null,
    heartbeat_at: now,
    created_at: before?.created_at ?? now,
    updated_at: now,
    manga_title: "",
    number: chapter.number,
    title: chapter.title,
    language: chapter.language,
    local_path: chapter.local_path,
    ...before,
    ...patch,
  };
  jobs.set(chapter.id, job);
  const event = { kind: kindOf(job.state, before?.state), job } as QueueEvent;
  for (const cb of listeners) cb(event);
}

function control(from: JobState[], to: JobState) {
  return async (id: number | null) => {
    const changed = [...jobs.values()]
      .filter((j) => (id === null || j.id === id) && from.includes(j.state))
      .map((j) => ({ ...j, state: to }));
    for (const job of changed) {
      jobs.set(job.chapter_id, job);
      for (const cb of listeners) cb({ kind: kindOf(to, undefined), job } as QueueEvent);
    }
    return changed;
  };
}

export const mockQueue: DownloadsBackend = {
  listDownloads: async () => [...jobs.values()],
  pauseDownload: control(["queued", "running"], "paused"),
  resumeDownload: control(["paused"], "queued"),
  cancelDownload: control(["queued", "running", "paused", "failed"], "cancelled"),
  retryDownload: control(["failed", "cancelled"], "queued"),
  async clearDownloads() {
    const gone = [...jobs.values()].filter((j) => j.state === "done" || j.state === "cancelled");
    for (const job of gone) {
      jobs.delete(job.chapter_id);
      for (const cb of listeners) cb({ kind: "removed", id: job.id });
    }
    return gone.length;
  },
  onQueueEvent(cb): Promise<Unlisten> {
    listeners.add(cb);
    return Promise.resolve(() => listeners.delete(cb));
  },
};
