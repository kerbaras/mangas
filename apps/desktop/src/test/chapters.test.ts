import { describe, expect, it } from "vitest";
import {
  chapterLabel,
  downloadable,
  exportable,
  selectionSummary,
  selectRange,
  sortChapters,
  toggle,
  toggleAll,
} from "@/lib/chapters";
import type { Chapter } from "@/lib/ipc";

const ch = (number: string, extra: Partial<Chapter> = {}): Chapter => ({
  id: `c${number}${extra.language ?? ""}`,
  manga_id: "m",
  title: null,
  language: "en",
  volume: "1",
  number,
  local_path: null,
  ...extra,
});

describe("sortChapters", () => {
  it("orders numerically, not lexically, and keeps 12.5 between 12 and 13", () => {
    const sorted = sortChapters([ch("13"), ch("2"), ch("12.5"), ch("10"), ch("12")]);
    expect(sorted.map((c) => c.number)).toEqual(["2", "10", "12", "12.5", "13"]);
  });

  it("sorts by volume first and puts chapters without a volume last", () => {
    const sorted = sortChapters([
      ch("1", { volume: null }),
      ch("9", { volume: "1" }),
      ch("1", { volume: "2" }),
    ]);
    expect(sorted.map((c) => `${c.volume}:${c.number}`)).toEqual(["1:9", "2:1", "null:1"]);
  });
});

describe("selection", () => {
  const ids = ["c1", "c2", "c3", "c4", "c5"];

  it("toggle adds and removes", () => {
    const on = toggle(new Set(), "c1");
    expect([...on]).toEqual(["c1"]);
    expect(toggle(on, "c1").size).toBe(0);
  });

  it("selectRange extends from the anchor in either direction", () => {
    expect([...selectRange(new Set(["c2"]), ids, "c2", "c4")]).toEqual(["c2", "c3", "c4"]);
    expect([...selectRange(new Set(["c4"]), ids, "c4", "c2")].sort()).toEqual(["c2", "c3", "c4"]);
  });

  it("selectRange falls back to toggle without an anchor", () => {
    expect([...selectRange(new Set(), ids, null, "c3")]).toEqual(["c3"]);
  });

  it("toggleAll selects everything visible, then clears only the visible ids", () => {
    const all = toggleAll(new Set(["other"]), ids);
    expect(all.size).toBe(6);
    const cleared = toggleAll(all, ids);
    expect([...cleared]).toEqual(["other"]);
  });
});

describe("downloadable / exportable", () => {
  const chapters = [ch("1", { local_path: "/d/1" }), ch("2"), ch("3")];
  const selected = new Set(["c1", "c2"]);

  it("splits the selection by what is on disk", () => {
    expect(downloadable(chapters, selected).map((c) => c.number)).toEqual(["2"]);
    expect(exportable(chapters, selected).map((c) => c.number)).toEqual(["1"]);
  });

  it("summarises the selection for the toolbar", () => {
    expect(selectionSummary(chapters, new Set())).toBe("No chapters selected");
    expect(selectionSummary(chapters, selected)).toBe("2 selected, 1 to download");
    expect(selectionSummary(chapters, new Set(["c1"]))).toBe("1 selected, all downloaded");
  });
});

describe("chapterLabel", () => {
  it("joins volume, chapter and title and skips blanks", () => {
    expect(chapterLabel(ch("12", { title: "The Brand" }))).toBe("Vol. 1 · Ch. 12 · The Brand");
    expect(chapterLabel(ch("3", { volume: null }))).toBe("Ch. 3");
  });
});
