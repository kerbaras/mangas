/**
 * Browser-mock download queue: keeps one job per chapter and emits the same `QueueEvent`s the
 * Rust worker does, including the queue-wide gate (pause all, `?held` for a data-folder move).
 */
import type { Chapter, Unlisten } from "./ipc";
import type {
  DownloadJob,
  DownloadsBackend,
  JobState,
  QueueEvent,
  QueueStatus,
} from "./ipc.downloads";
import { mockFlags } from "./mock-flags";

const listeners = new Set<(e: QueueEvent) => void>();
const jobs = new Map<string, DownloadJob>();
let nextId = 1;
const gate: QueueStatus = { paused: false, held: mockFlags.held };

const kindOf = (state: JobState, before: JobState | undefined) =>
  state === "running" ? (before === "running" ? "progress" : "started") : state;

const emit = (event: QueueEvent) => {
  for (const cb of listeners) cb(event);
};

/** The state of `chapter`'s job, so the simulated download stops once paused or cancelled. */
export const mockJobState = (chapterId: string) => jobs.get(chapterId)?.state;
/** Pause all or a move hold: nothing new starts. */
export const mockGateClosed = () => gate.paused || gate.held;
export const mockJobs = () => [...jobs.values()];

/** Stores `job` as is and tells the listeners. */
export function putJob(job: DownloadJob) {
  const before = jobs.get(job.chapter_id);
  jobs.set(job.chapter_id, job);
  emit({ kind: kindOf(job.state, before?.state), job } as QueueEvent);
}

/** Creates or updates the job of `chapter` and tells the listeners. */
export function emitJob(chapter: Chapter, source: string, patch: Partial<DownloadJob>) {
  const before = jobs.get(chapter.id);
  const now = Date.now();
  putJob({
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
    stale_lease: false,
    stalled: false,
    force: false,
    created_at: before?.created_at ?? now,
    manga_title: "",
    number: chapter.number,
    title: chapter.title,
    language: chapter.language,
    local_path: chapter.local_path,
    group: null,
    ...before,
    ...patch,
    updated_at: now,
  });
}

function setGate(paused: boolean) {
  if (gate.paused === paused) return;
  gate.paused = paused;
  emit({ kind: "gate", ...gate });
}

function control(from: JobState[], to: JobState, all?: () => void) {
  return async (id: number | null) => {
    if (id === null) all?.();
    const changed = [...jobs.values()]
      .filter((j) => (id === null || j.id === id) && from.includes(j.state))
      .map((j) => ({ ...j, state: to, updated_at: Date.now() }));
    if (id !== null && changed.length === 0) throw new Error(`download ${id}: nothing to do`);
    for (const job of changed) {
      jobs.set(job.chapter_id, job);
      emit({ kind: kindOf(to, undefined), job } as QueueEvent);
    }
    return changed;
  };
}

function remove(job: DownloadJob) {
  jobs.delete(job.chapter_id);
  emit({ kind: "removed", id: job.id });
}

export const mockQueue: DownloadsBackend = {
  listDownloads: async () => [...jobs.values()],
  pauseDownload: control(["queued", "running"], "paused", () => setGate(true)),
  resumeDownload: control(["paused"], "queued", () => setGate(false)),
  cancelDownload: control(["queued", "running", "paused", "failed"], "cancelled"),
  retryDownload: control(["failed", "cancelled"], "queued"),
  async clearDownloads() {
    const gone = [...jobs.values()].filter((j) => j.state === "done" || j.state === "cancelled");
    gone.forEach(remove);
    return gone.length;
  },
  async dismissDownload(id) {
    const job = [...jobs.values()].find((j) => j.id === id);
    if (!job) throw new Error(`not found: download ${id}`);
    if (!["done", "failed", "cancelled"].includes(job.state))
      throw new Error(`download ${id} is ${job.state}; pause or cancel it first`);
    remove(job);
  },
  queueStatus: async () => ({ ...gate }),
  onQueueEvent(cb): Promise<Unlisten> {
    listeners.add(cb);
    return Promise.resolve(() => listeners.delete(cb));
  },
};
