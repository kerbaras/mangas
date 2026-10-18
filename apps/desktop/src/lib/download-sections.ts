/** How the Downloads screen groups and labels jobs. Pure. */
import { type DownloadJob, isRefused, isUnlisted, type JobState } from "./ipc.downloads";

export type SectionKey = "active" | "queued" | "failed" | "completed";

export const SECTIONS: { key: SectionKey; title: string }[] = [
  { key: "active", title: "Active" },
  { key: "queued", title: "Queued" },
  { key: "failed", title: "Failed" },
  { key: "completed", title: "Completed" },
];

const SECTION_OF: Record<JobState, SectionKey> = {
  running: "active",
  paused: "active",
  queued: "queued",
  failed: "failed",
  done: "completed",
  cancelled: "completed",
};

export const sectionOf = (job: DownloadJob): SectionKey => SECTION_OF[job.state];

/**
 * Active and queued keep queue order (the worker picks by `created_at`, which a retry resets);
 * failed and completed show the latest first.
 */
export function groupJobs(jobs: Iterable<DownloadJob>): Record<SectionKey, DownloadJob[]> {
  const out: Record<SectionKey, DownloadJob[]> = {
    active: [],
    queued: [],
    failed: [],
    completed: [],
  };
  for (const job of jobs) out[sectionOf(job)].push(job);
  const inQueue = (a: DownloadJob, b: DownloadJob) => a.created_at - b.created_at || a.id - b.id;
  const latest = (a: DownloadJob, b: DownloadJob) => b.updated_at - a.updated_at || b.id - a.id;
  out.active.sort(inQueue);
  out.queued.sort(inQueue);
  out.failed.sort(latest);
  out.completed.sort(latest);
  return out;
}

/** Running or queued: what the nav badge counts. */
export const pendingCount = (jobs: Iterable<DownloadJob>) =>
  [...jobs].filter((j) => j.state === "running" || j.state === "queued").length;

/** The state word every row shows; a running job is "Starting" until its pages are listed. */
export function stateLabel(job: DownloadJob): string {
  switch (job.state) {
    case "running":
      if (job.stale_lease) return "Stale, will resume";
      if (job.stalled) return "Stalled, waiting to resume";
      return job.pages_total === 0 ? "Starting" : "Downloading";
    case "queued":
      return job.error ? "Retrying" : "Queued";
    case "paused":
      return "Paused";
    case "failed":
      return "Failed";
    case "cancelled":
      return "Cancelled";
    case "done":
      return "Done";
  }
}

/** What the user can do with a job from its row. */
export type JobAction = "pause" | "resume" | "cancel" | "retry" | "dismiss" | "open";

export function jobActions(job: DownloadJob): JobAction[] {
  switch (job.state) {
    case "running":
      return ["pause", "cancel"];
    case "paused":
      return ["resume", "cancel"];
    case "queued":
      return ["cancel"];
    case "failed":
    case "cancelled":
      return isUnlisted(job) || isRefused(job.error) ? ["dismiss"] : ["retry", "dismiss"];
    case "done":
      return job.local_path ? ["open", "dismiss"] : ["dismiss"];
  }
}

/** "Berserk · Chapter 12", falling back to the chapter alone for jobs without a title. */
export const jobTitle = (job: DownloadJob) =>
  job.manga_title ? `${job.manga_title} · Chapter ${job.number}` : `Chapter ${job.number}`;

/** "1.4 pages/s", or null while it is not known. */
/** Running and still heard from: the only rows whose bar and speed move. */
export const isMoving = (job: DownloadJob) =>
  job.state === "running" && !job.stalled && !job.stale_lease;

export function speedLabel(pagesPerSecond: number | null): string | null {
  if (pagesPerSecond === null || pagesPerSecond <= 0) return null;
  const digits = pagesPerSecond < 10 ? 1 : 0;
  return `${pagesPerSecond.toFixed(digits)} pages/s`;
}

/** "3 active · 12 queued" for the header, or null with nothing pending. */
export function summary(groups: Record<SectionKey, DownloadJob[]>): string | null {
  const parts = [
    groups.active.length && `${groups.active.length} active`,
    groups.queued.length && `${groups.queued.length} queued`,
    groups.failed.length && `${groups.failed.length} failed`,
  ].filter(Boolean);
  return parts.length ? parts.join(" · ") : null;
}
