/**
 * Download queue part of the IPC contract (`mangas_core::download::queue`). Jobs persist across
 * restarts and are shared with the CLI; every change arrives as a `QueueEvent`.
 */
import type { DownloadProgress, Unlisten } from "./ipc";
import { backend } from "./ipc";

/** Mirrors `mangas_core::download::queue::JobState`. */
export type JobState = "queued" | "running" | "paused" | "done" | "failed" | "cancelled";

/** Mirrors `mangas_core::download::queue::Job`; times are unix milliseconds. */
export type DownloadJob = {
  id: number;
  source: string;
  manga_id: string;
  chapter_id: string;
  state: JobState;
  attempts: number;
  /** Last failure; also set on a queued job waiting to retry. */
  error: string | null;
  pages_done: number;
  /** 0 until the source listed the chapter's pages. */
  pages_total: number;
  /** A queued job waits until then (retry backoff). */
  not_before: number;
  owner: string | null;
  heartbeat_at: number;
  /** Running, but its process died (lease expired): a worker queues it again soon. */
  stale_lease: boolean;
  created_at: number;
  updated_at: number;
  manga_title: string;
  number: string;
  title: string | null;
  language: string;
  local_path: string | null;
};

/** Mirrors `mangas_core::download::queue::QueueEvent` (serde tag = "kind"). */
export type QueueEvent =
  | {
      kind: "queued" | "started" | "progress" | "paused" | "done" | "failed" | "cancelled";
      job: DownloadJob;
    }
  | { kind: "removed"; id: number };

export type DownloadsBackend = {
  /** Every job in queue order. */
  listDownloads(): Promise<DownloadJob[]>;
  /** `id` null acts on every job the action applies to; resolves to the jobs changed. */
  pauseDownload(id: number | null): Promise<DownloadJob[]>;
  resumeDownload(id: number | null): Promise<DownloadJob[]>;
  cancelDownload(id: number | null): Promise<DownloadJob[]>;
  retryDownload(id: number | null): Promise<DownloadJob[]>;
  /** Removes done and cancelled jobs; resolves to how many. */
  clearDownloads(): Promise<number>;
  onQueueEvent(cb: (e: QueueEvent) => void): Promise<Unlisten>;
};

export const listDownloads = async () => (await backend()).listDownloads();
export const pauseDownload = async (id: number | null) => (await backend()).pauseDownload(id);
export const resumeDownload = async (id: number | null) => (await backend()).resumeDownload(id);
export const cancelDownload = async (id: number | null) => (await backend()).cancelDownload(id);
export const retryDownload = async (id: number | null) => (await backend()).retryDownload(id);
export const clearDownloads = async () => (await backend()).clearDownloads();
export const onQueueEvent = async (cb: (e: QueueEvent) => void) =>
  (await backend()).onQueueEvent(cb);

const FINAL: JobState[] = ["done", "failed", "cancelled"];

/** What a chapter row shows for its job. `error` only once the job has given up. */
export const toDownloadProgress = (job: DownloadJob): DownloadProgress => ({
  chapter_id: job.chapter_id,
  state: job.state,
  page: job.pages_done,
  pages: job.pages_total,
  done: FINAL.includes(job.state),
  error: job.state === "failed" ? (job.error ?? "failed") : null,
  retrying: job.state === "queued" ? job.error : null,
});

/** Queued or running: counted in the nav badge. Paused ones wait for the user. */
export const isActive = (p: DownloadProgress) => p.state === "queued" || p.state === "running";
