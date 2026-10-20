import { afterEach, describe, expect, it, vi } from "vitest";
import type { ExportProgress } from "@/lib/ipc";
import { PROGRESS_MS, throttleProgress } from "@/lib/progress-throttle";

const pages = (current: number): ExportProgress => ({
  stage: "Cutting pages",
  current,
  total: null,
});

describe("export progress throttle", () => {
  afterEach(() => vi.useRealTimers());

  it("folds one event per page into about 4 updates a second", () => {
    vi.useFakeTimers();
    const seen: ExportProgress[] = [];
    const t = throttleProgress((p) => seen.push(p));
    for (let page = 1; page <= 100; page++) {
      t.push(pages(page));
      vi.advanceTimersByTime(10);
    }
    vi.advanceTimersByTime(PROGRESS_MS);
    expect(seen.length).toBeGreaterThanOrEqual(4);
    expect(seen.length).toBeLessThanOrEqual(6);
    expect(seen.at(-1)?.current).toBe(100);
  });

  it("delivers a new stage, a notice and the last step at once", () => {
    vi.useFakeTimers();
    const seen: ExportProgress[] = [];
    const t = throttleProgress((p) => seen.push(p));
    t.push(pages(1));
    t.push(pages(2));
    t.push({ stage: "Writing file", current: 0, total: 40 });
    t.push({ stage: "Writing file", current: 40, total: 40 });
    t.push({ stage: "Writing file", current: 40, total: 40, notice: "note" });
    expect(seen.map((p) => [p.stage, p.current])).toEqual([
      ["Cutting pages", 1],
      ["Writing file", 0],
      ["Writing file", 40],
      ["Writing file", 40],
    ]);
  });

  it("drops a pending step on reset", () => {
    vi.useFakeTimers();
    const seen: ExportProgress[] = [];
    const t = throttleProgress((p) => seen.push(p));
    t.push(pages(1));
    t.push(pages(2));
    t.reset();
    vi.advanceTimersByTime(PROGRESS_MS * 2);
    expect(seen.map((p) => p.current)).toEqual([1]);
  });
});
