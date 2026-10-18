import { describe, expect, it } from "vitest";
import type { StorageUsage } from "@/lib/ipc.storage";
import { formatBytes, plural, restoreSummary, share, usageSegments } from "@/lib/storage";

describe("storage helpers", () => {
  it("formats sizes like the CLI", () => {
    expect(formatBytes(0)).toBe("0 B");
    expect(formatBytes(999)).toBe("999 B");
    expect(formatBytes(1_500)).toBe("1.50 KB");
    expect(formatBytes(12_340_000)).toBe("12.3 MB");
    expect(formatBytes(204_000_000_000)).toBe("204 GB");
  });

  it("splits usage into bar segments with library and logs together", () => {
    const usage: StorageUsage = {
      data_dir: "/d",
      chapters: 600,
      models: 300,
      covers: 50,
      library: 30,
      logs: 20,
      titles: [],
    };
    const segments = usageSegments(usage);
    expect(segments.map((s) => [s.key, s.bytes])).toEqual([
      ["chapters", 600],
      ["models", 300],
      ["covers", 50],
      ["library", 50],
    ]);
    expect(share(600, 1000)).toBe(60);
    expect(share(1, 0)).toBe(0);
  });

  it("pluralises counts", () => {
    expect(plural(1, "imported title was", "imported titles were")).toBe("1 imported title was");
    expect(plural(2, "imported title was", "imported titles were")).toBe("2 imported titles were");
    expect(plural(1, "title")).toBe("1 title");
  });

  it("summarises an import", () => {
    expect(
      restoreSummary({
        added: 1,
        already: 3,
        skipped: [{ title: "Elsewhere", reason: "x" }],
        settings: true,
      }),
    ).toBe(
      "Followed 1 new title. 3 titles already in your library. Skipped Elsewhere: provider not supported. Preferences restored.",
    );
    expect(restoreSummary({ added: 0, already: 0, skipped: [], settings: false })).toBe(
      "Followed 0 new titles.",
    );
  });
});
