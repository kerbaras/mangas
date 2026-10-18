import { describe, expect, it } from "vitest";
import {
  groupJobs,
  jobActions,
  pendingCount,
  speedLabel,
  stateLabel,
  summary,
} from "@/lib/download-sections";
import { type JobState, UNLISTED } from "@/lib/ipc.downloads";
import { makeJob } from "./job-fixture";

describe("downloads sections", () => {
  it("groups by state: running and paused are active, cancelled is completed", () => {
    const states: JobState[] = ["queued", "running", "paused", "failed", "done", "cancelled"];
    const g = groupJobs(states.map((state, i) => makeJob({ id: i + 1, state })));
    expect(g.active.map((j) => j.state)).toEqual(["running", "paused"]);
    expect(g.queued.map((j) => j.state)).toEqual(["queued"]);
    expect(g.failed.map((j) => j.state)).toEqual(["failed"]);
    expect(g.completed.map((j) => j.state).sort()).toEqual(["cancelled", "done"]);
  });

  it("keeps queue order for queued jobs and shows the latest finished first", () => {
    const g = groupJobs([
      makeJob({ id: 3 }),
      makeJob({ id: 1 }),
      makeJob({ id: 4, state: "done", updated_at: 10 }),
      makeJob({ id: 5, state: "done", updated_at: 30 }),
    ]);
    expect(g.queued.map((j) => j.id)).toEqual([1, 3]);
    expect(g.completed.map((j) => j.id)).toEqual([5, 4]);
  });

  it("puts a retried job at the back of the queue, where the worker picks it", () => {
    const g = groupJobs([makeJob({ id: 1, created_at: 50 }), makeJob({ id: 2, created_at: 20 })]);
    expect(g.queued.map((j) => j.id)).toEqual([2, 1]);
  });

  it("labels every state, with starting before the pages are listed", () => {
    const label = (patch: Parameters<typeof makeJob>[0]) => stateLabel(makeJob(patch));
    expect(label({ state: "queued" })).toBe("Queued");
    expect(label({ state: "queued", error: "HTTP 503" })).toBe("Retrying");
    expect(label({ state: "running" })).toBe("Starting");
    expect(label({ state: "running", pages_total: 20 })).toBe("Downloading");
    expect(label({ state: "paused" })).toBe("Paused");
    expect(label({ state: "failed" })).toBe("Failed");
    expect(label({ state: "cancelled" })).toBe("Cancelled");
    expect(label({ state: "done" })).toBe("Done");
  });

  it("offers the actions each state allows; an unlisted chapter can only be cleared", () => {
    expect(jobActions(makeJob({ state: "running" }))).toEqual(["pause", "cancel"]);
    expect(jobActions(makeJob({ state: "paused" }))).toEqual(["resume", "cancel"]);
    expect(jobActions(makeJob({ state: "queued" }))).toEqual(["cancel"]);
    expect(jobActions(makeJob({ state: "failed", error: "HTTP 429" }))).toEqual([
      "retry",
      "dismiss",
    ]);
    expect(jobActions(makeJob({ state: "failed", error: UNLISTED }))).toEqual(["dismiss"]);
    expect(jobActions(makeJob({ state: "done", local_path: "/d/7" }))).toEqual(["open", "dismiss"]);
  });

  it("counts running and queued jobs for the badge, not paused or finished ones", () => {
    const states: JobState[] = ["queued", "running", "paused", "failed", "done", "queued"];
    expect(pendingCount(states.map((state, i) => makeJob({ id: i, state })))).toBe(3);
  });

  it("formats speed and the header summary", () => {
    expect(speedLabel(null)).toBeNull();
    expect(speedLabel(0)).toBeNull();
    expect(speedLabel(1.25)).toBe("1.3 pages/s");
    expect(speedLabel(12.4)).toBe("12 pages/s");
    const g = groupJobs([makeJob({ state: "running" }), makeJob({ id: 2 }), makeJob({ id: 3 })]);
    expect(summary(g)).toBe("1 active · 2 queued");
    expect(summary(groupJobs([makeJob({ state: "done" })]))).toBeNull();
  });
});
