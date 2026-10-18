/**
 * The desktop's copy of the persistent queue, folded from `QueueEvent`s and the startup snapshot.
 * Pure: the provider owns the state and the clock.
 */
import type { DownloadJob, QueueEvent, QueueStatus } from "./ipc.downloads";

/** Pages done at the moment a progress event arrived. */
export type Sample = { at: number; pages: number };

export type QueueState = {
  jobs: ReadonlyMap<number, DownloadJob>;
  gate: QueueStatus;
  /** Recent progress of each running job, oldest first, for its speed. */
  samples: ReadonlyMap<number, readonly Sample[]>;
};

export const emptyQueue: QueueState = {
  jobs: new Map(),
  gate: { paused: false, held: false },
  samples: new Map(),
};

/** Speed looks at this much of the latest progress. */
export const SPEED_WINDOW_MS = 8000;

/** Keeps the samples of a running job inside the window; a reset or a stop starts over. */
export function track(
  samples: readonly Sample[] | undefined,
  job: DownloadJob,
  at: number,
): Sample[] | undefined {
  if (job.state !== "running" || job.pages_total === 0) return undefined;
  const kept = (samples ?? []).filter((s) => s.pages <= job.pages_done);
  const last = kept[kept.length - 1];
  if (last?.pages === job.pages_done) return kept;
  return [...kept, { at, pages: job.pages_done }].filter((s) => at - s.at <= SPEED_WINDOW_MS);
}

/** Pages per second over the samples, or null until two of them are far enough apart. */
export function pagesPerSecond(samples: readonly Sample[] | undefined): number | null {
  if (!samples || samples.length < 2) return null;
  const first = samples[0];
  const last = samples[samples.length - 1];
  const seconds = (last.at - first.at) / 1000;
  return seconds >= 0.5 ? (last.pages - first.pages) / seconds : null;
}

function withJob(state: QueueState, job: DownloadJob, at: number): QueueState {
  const jobs = new Map(state.jobs).set(job.id, job);
  const samples = new Map(state.samples);
  const next = track(state.samples.get(job.id), job, at);
  if (next) samples.set(job.id, next);
  else samples.delete(job.id);
  return { ...state, jobs, samples };
}

function without(state: QueueState, id: number): QueueState {
  if (!state.jobs.has(id)) return state;
  const jobs = new Map(state.jobs);
  const samples = new Map(state.samples);
  jobs.delete(id);
  samples.delete(id);
  return { ...state, jobs, samples };
}

/** Folds one event that arrived at `at`. */
export function applyEvent(state: QueueState, event: QueueEvent, at: number): QueueState {
  switch (event.kind) {
    case "removed":
      return without(state, event.id);
    case "gate":
      return { ...state, gate: { paused: event.paused, held: event.held } };
    default:
      return withJob(state, event.job, at);
  }
}

/** Folds a batch of `[event, arrival]` pairs in order. */
export const applyEvents = (state: QueueState, batch: readonly [QueueEvent, number][]) =>
  batch.reduce((s, [event, at]) => applyEvent(s, event, at), state);

/**
 * Folds the startup snapshot. Events can arrive before it resolves, and they are newer: a job
 * in `fromEvents` keeps what its events said, and so does the gate once an event reported it.
 */
export function applySnapshot(
  state: QueueState,
  snapshot: { jobs?: readonly DownloadJob[]; gate?: QueueStatus },
  fromEvents: { jobs: ReadonlySet<number>; gate: boolean },
): QueueState {
  const fresh = (snapshot.jobs ?? []).filter((j) => !fromEvents.jobs.has(j.id));
  const jobs = new Map([...fresh.map((j) => [j.id, j] as const), ...state.jobs]);
  const gate = snapshot.gate && !fromEvents.gate ? snapshot.gate : state.gate;
  return { ...state, jobs, gate };
}

/** Chapter id → its latest job (a chapter downloaded again has a newer job). */
export function jobsByChapter(jobs: ReadonlyMap<number, DownloadJob>): Map<string, DownloadJob> {
  const out = new Map<string, DownloadJob>();
  for (const job of jobs.values()) {
    const seen = out.get(job.chapter_id);
    if (!seen || seen.id < job.id) out.set(job.chapter_id, job);
  }
  return out;
}
