import { describe, expect, it } from "vitest";
import { importSummary } from "@/lib/import-summary";
import type { ImportReport } from "@/lib/ipc";
import { mockImport } from "@/lib/mock-import";

const report = (imported: ImportReport["imported"], skipped = 0): ImportReport => ({
  manga: mockImport(["/m/Berserk"], undefined, "/d").report.manga,
  imported,
  skipped: Array.from({ length: skipped }, (_, i) => ({ name: `x${i}`, reason: "r" })),
});
const ch = (number: string, pages: number) => ({ number, title: null, pages, path: "/p" });

describe("importSummary", () => {
  it("counts chapters, pages and skipped entries", () => {
    expect(importSummary(report([ch("1", 10), ch("2", 12)], 1))).toBe(
      "Imported 2 chapters (22 pages) of Berserk · 1 skipped",
    );
  });
  it("uses singulars and flags unnumbered chapters", () => {
    expect(importSummary(report([ch("", 1)]))).toBe(
      "Imported 1 chapter (1 page) of Berserk · 1 without a number",
    );
  });
  it("says when nothing new was found", () => {
    expect(importSummary(report([], 2))).toBe("Nothing new in Berserk · 2 skipped");
  });
});

describe("mockImport", () => {
  it("builds a local manga slugged from the folder name", () => {
    const { report: r, chapters } = mockImport(["/m/Blame! Master"], undefined, "/d");
    expect(r.manga).toMatchObject({ id: "blame-master", source: "local", title: "Blame! Master" });
    expect(chapters.map((c) => c.local_path)).toContain("/d/local/blame-master/1");
  });
});
