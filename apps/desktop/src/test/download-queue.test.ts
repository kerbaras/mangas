import { describe, expect, it } from "vitest";
import { type DownloadJob, isActive, toDownloadProgress } from "@/lib/ipc.downloads";

const job = (patch: Partial<DownloadJob>): DownloadJob => ({
  id: 1,
  source: "mangadex",
  manga_id: "m",
  chapter_id: "c1",
  state: "queued",
  attempts: 0,
  error: null,
  pages_done: 0,
  pages_total: 0,
  not_before: 0,
  owner: null,
  heartbeat_at: 0,
  stale_lease: false,
  stalled: false,
  force: false,
  created_at: 0,
  updated_at: 0,
  manga_title: "Kaiju",
  number: "7",
  title: null,
  language: "en",
  local_path: null,
  ...patch,
});

describe("queue jobs as chapter progress", () => {
  it("maps a running job to its page count", () => {
    expect(toDownloadProgress(job({ state: "running", pages_done: 3, pages_total: 20 }))).toEqual({
      chapter_id: "c1",
      state: "running",
      page: 3,
      pages: 20,
      done: false,
      error: null,
      retrying: null,
    });
  });

  it("reports an error only once the job gave up; a waiting retry keeps it aside", () => {
    const waiting = toDownloadProgress(job({ state: "queued", error: "HTTP 429" }));
    expect(waiting).toMatchObject({ done: false, error: null, retrying: "HTTP 429" });
    const failed = toDownloadProgress(job({ state: "failed", error: "HTTP 429" }));
    expect(failed).toMatchObject({ done: true, error: "HTTP 429", retrying: null });
  });

  it("treats done and cancelled as finished without an error", () => {
    expect(toDownloadProgress(job({ state: "done" }))).toMatchObject({ done: true, error: null });
    expect(toDownloadProgress(job({ state: "cancelled" }))).toMatchObject({ done: true });
  });

  it("counts queued and running jobs as active, not paused ones", () => {
    const states = ["queued", "running", "paused", "done", "failed"] as const;
    const active = states.filter((state) => isActive(toDownloadProgress(job({ state }))));
    expect(active).toEqual(["queued", "running"]);
  });
});
