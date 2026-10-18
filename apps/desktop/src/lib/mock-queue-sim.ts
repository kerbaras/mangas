/**
 * A running queue for the browser mock, so `pnpm -C apps/desktop dev:web` shows the Downloads
 * screen at work: 50-odd jobs across every state, two running at a time, honouring pause all.
 */
import type { DownloadJob, DownloadsBackend, JobState } from "./ipc.downloads";
import { UNLISTED } from "./ipc.downloads";
import { sampleLibrary } from "./mock-data";
import { mockFlags } from "./mock-flags";
import { mockGateClosed, mockJobs, mockQueue, putJob } from "./mock-queue";

export { emitJob, mockJobState } from "./mock-queue";

const DATA_DIR = "~/.local/share/mangas";
const PREFIX = "seed:";
const RUNNING = 2;
const TICK_MS = 400;

type Seed = { state: JobState; done?: number; total?: number; error?: string };

/** Index → what that seeded job starts as; anything not listed is queued. */
const starts: Record<number, Seed> = {
  0: { state: "running", done: 6, total: 22 },
  1: { state: "running", done: 15, total: 19 },
  2: { state: "paused", done: 4, total: 24 },
  3: { state: "failed", done: 9, total: 20, error: "HTTP 429 from source: too many requests" },
  4: { state: "failed", error: UNLISTED },
  5: { state: "cancelled", done: 2, total: 18 },
  6: { state: "queued", error: "network: connection reset by peer" },
};
const DONE = [7, 8, 9, 10, 11, 12];

function seedJob(i: number, now: number): DownloadJob {
  const manga = sampleLibrary[i % sampleLibrary.length];
  const n = 40 + i;
  const start = DONE.includes(i) ? { state: "done" as const, done: 20, total: 20 } : starts[i];
  const state = start?.state ?? "queued";
  const path = `${DATA_DIR}/${manga.source}/${manga.id}/${String(n).padStart(3, "0")}`;
  return {
    id: 1000 + i,
    source: manga.source,
    manga_id: manga.id,
    chapter_id: `${PREFIX}${manga.id}:${n}`,
    state,
    attempts: state === "queued" && !start ? 0 : 1,
    error: start?.error ?? null,
    pages_done: start?.done ?? 0,
    pages_total: start?.total ?? 0,
    not_before: start?.error && state === "queued" ? now + 30_000 : 0,
    owner: null,
    heartbeat_at: now,
    stale_lease: false,
    stalled: false,
    force: false,
    created_at: now - (60 - i) * 60_000,
    updated_at: now - (60 - i) * 1000,
    manga_title: manga.title,
    number: String(n),
    title: null,
    language: "en",
    local_path: state === "done" ? path : null,
  };
}

/** One step: running jobs move on, free slots pick up the next queued job. */
function tick() {
  if (mockGateClosed()) return;
  const now = Date.now();
  const seeded = mockJobs().filter((j) => j.chapter_id.startsWith(PREFIX));
  for (const job of seeded.filter((j) => j.state === "running")) {
    if (job.pages_total === 0) {
      putJob({ ...job, pages_total: 16 + (job.id % 9), heartbeat_at: now, updated_at: now });
      continue;
    }
    const pages = Math.min(job.pages_total, job.pages_done + 1 + Math.round(Math.random()));
    const done = pages === job.pages_total;
    const local_path = done ? `${DATA_DIR}/${job.source}/${job.manga_id}/${job.number}` : null;
    putJob({
      ...job,
      state: done ? "done" : "running",
      pages_done: pages,
      local_path,
      updated_at: now,
    });
  }
  const running = mockJobs().filter((j) => j.state === "running").length;
  const next = seeded
    .filter((j) => j.state === "queued" && j.not_before <= now)
    .sort((a, b) => a.id - b.id)
    .slice(0, Math.max(0, RUNNING - running));
  for (const job of next) putJob({ ...job, state: "running", attempts: job.attempts + 1 });
}

let started = false;

/** Seeds `count` jobs and starts the clock (once). */
export function startMockQueue(count = 54) {
  if (started) return;
  started = true;
  const now = Date.now();
  for (let i = 0; i < count; i++) putJob(seedJob(i, now));
  setInterval(tick, TICK_MS);
}

/** The mock queue, running the seeded jobs unless `?quiet-queue` (and never under tests). */
export function simulatedQueue(): DownloadsBackend {
  const mode = (import.meta as ImportMeta & { env?: { MODE?: string } }).env?.MODE;
  if (!mockFlags.quietQueue && mode !== "test") startMockQueue();
  return mockQueue;
}
