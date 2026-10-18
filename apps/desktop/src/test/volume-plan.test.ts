import { describe, expect, it } from "vitest";
import type { Chapter } from "@/lib/ipc";
import { toExportProgress } from "@/lib/ipc.wire";
import { mockPlan, planSummary } from "@/lib/volume-plan";

const ch = (number: string, volume: string | null, downloaded = true): Chapter => ({
  id: `c${number}`,
  manga_id: "m",
  title: null,
  language: "en",
  volume,
  number,
  local_path: downloaded ? `/d/${number}` : null,
});

const chapters = [ch("3", "10"), ch("1", "2"), ch("2", "2"), ch("4", null), ch("5", "10", false)];

describe("volume plan (browser mock)", () => {
  it("groups downloaded chapters by provider volume, numerically, loose ones last", () => {
    const plans = mockPlan("Solo: Rise", chapters, { kind: "by_volume" });
    expect(plans.map((p) => p.file_name)).toEqual([
      "Solo Rise v2.epub",
      "Solo Rise v10.epub",
      "Solo Rise ch 4.epub",
    ]);
    expect(plans[0].chapters.map((c) => c.number)).toEqual(["1", "2"]);
  });

  it("cuts every N chapters and names a single book with the given title", () => {
    expect(mockPlan("S", chapters, { kind: "every", size: 3 }).map((p) => p.title)).toEqual([
      "S v1",
      "S v2",
    ]);
    const [one] = mockPlan("S", chapters, { kind: "single" }, " Best of ");
    expect(one).toMatchObject({ title: "Best of", file_name: "Best of.epub" });
    expect(mockPlan("S", chapters, { kind: "single" })[0].title).toBe("S ch 1-4");
  });

  it("summarises a book's chapter range", () => {
    const [v2] = mockPlan("S", chapters, { kind: "by_volume" });
    expect(planSummary(v2)).toBe("Chapters 1–2 · 2 chapters");
    expect(planSummary({ ...v2, chapters: [chapters[0]] })).toBe("Chapter 3");
  });

  it("reads the volume event of a multi-book export", () => {
    expect(toExportProgress({ event: "volume", index: 1, total: 3, title: "S v2" })).toEqual({
      stage: "Book 2 of 3: S v2",
      current: 0,
      total: null,
    });
  });
});
