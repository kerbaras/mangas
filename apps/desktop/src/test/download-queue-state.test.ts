import { describe, expect, it } from "vitest";
import {
  applyEvent,
  applyEvents,
  applySnapshot,
  emptyQueue,
  jobsByChapter,
  pagesPerSecond,
  SPEED_WINDOW_MS,
  track,
} from "@/lib/download-queue";
import { makeJob } from "./job-fixture";

const running = (pages_done: number) => makeJob({ state: "running", pages_done, pages_total: 20 });

describe("speed from progress deltas", () => {
  it("is pages over seconds between the oldest and newest sample", () => {
    const samples = [0, 1000, 2000].reduce(
      (s, at, i) => track(s, running(i * 3), at),
      undefined as ReturnType<typeof track>,
    );
    expect(pagesPerSecond(samples)).toBe(3);
  });

  it("is unknown with one sample or samples too close together", () => {
    expect(pagesPerSecond(track(undefined, running(2), 0))).toBeNull();
    expect(
      pagesPerSecond([
        { at: 0, pages: 0 },
        { at: 100, pages: 2 },
      ]),
    ).toBeNull();
  });

  it("forgets samples outside the window, after a restart and once not running", () => {
    const old = track(undefined, running(1), 0);
    expect(track(old, running(5), SPEED_WINDOW_MS + 1)).toEqual([
      { at: SPEED_WINDOW_MS + 1, pages: 5 },
    ]);
    expect(track([{ at: 0, pages: 9 }], running(2), 500)).toEqual([{ at: 500, pages: 2 }]);
    expect(track(old, running(1), SPEED_WINDOW_MS + 1)).toEqual([
      { at: SPEED_WINDOW_MS + 1, pages: 1 },
    ]);
    expect(track(old, makeJob({ state: "paused", pages_done: 3 }), 900)).toBeUndefined();
    expect(track(undefined, makeJob({ state: "running" }), 0)).toBeUndefined();
  });
});

describe("queue state", () => {
  it("adds, replaces and removes jobs and follows the gate", () => {
    let q = applyEvents(emptyQueue, [
      [{ kind: "queued", job: makeJob() }, 0],
      [{ kind: "started", job: running(0) }, 10],
      [{ kind: "gate", paused: true, held: false }, 20],
    ]);
    expect(q.jobs.get(1)?.state).toBe("running");
    expect(q.gate).toEqual({ paused: true, held: false });
    q = applyEvent(q, { kind: "removed", id: 1 }, 30);
    expect(q.jobs.size).toBe(0);
    expect(q.samples.size).toBe(0);
  });

  it("keeps what events said over a snapshot that resolves later", () => {
    const q = applyEvent(emptyQueue, { kind: "paused", job: makeJob({ state: "paused" }) }, 0);
    const gated = applyEvent(q, { kind: "gate", paused: true, held: false }, 0);
    const next = applySnapshot(
      gated,
      { jobs: [running(4), makeJob({ id: 2 })], gate: { paused: false, held: false } },
      { jobs: new Set([1]), gate: true },
    );
    expect(next.jobs.get(1)?.state).toBe("paused");
    expect(next.jobs.get(2)?.state).toBe("queued");
    expect(next.gate.paused).toBe(true);
  });

  it("takes the snapshot gate when no event reported it", () => {
    const next = applySnapshot(
      emptyQueue,
      { gate: { paused: false, held: true } },
      { jobs: new Set(), gate: false },
    );
    expect(next.gate.held).toBe(true);
  });

  it("maps each chapter to its newest job", () => {
    const jobs = new Map([
      [1, makeJob({ id: 1, chapter_id: "c", state: "done" })],
      [5, makeJob({ id: 5, chapter_id: "c", state: "running" })],
    ]);
    expect(jobsByChapter(jobs).get("c")?.id).toBe(5);
  });
});
