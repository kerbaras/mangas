import { describe, expect, it } from "vitest";
import { exportTitle, joinPath, parentDir } from "@/lib/export-name";
import type { Chapter, Manga } from "@/lib/ipc";
import { exportFileName } from "@/lib/ipc.volumes";

const manga = (title: string): Manga => ({
  id: title.toLowerCase(),
  source: "mangadex",
  title,
  description: "",
  cover_url: null,
  status: "ongoing",
});
const chapter = (number: string, title: string | null = null): Chapter => ({
  id: `c${number}`,
  manga_id: "m",
  title,
  language: "en",
  volume: null,
  number,
  local_path: `/dl/${number}`,
});

describe("exportTitle", () => {
  it("uses the chapter range of a single manga, sorted numerically", () => {
    const picks = [
      { manga: manga("Solo Leveling"), chapters: ["10", "2", "3"].map((n) => chapter(n)) },
    ];
    expect(exportTitle(picks)).toBe("Solo Leveling ch 2-10 (3 ch)");
    expect(exportTitle([{ manga: manga("ORV"), chapters: [chapter("262")] }])).toBe("ORV ch 262");
  });
  it("names season-numbered webtoon episodes by their season tag", () => {
    const tog = manga("Tower of God");
    const s1 = chapter("2", "S1 Ep. 1 - 1F.Headon's Floor");
    const s2 = chapter("81", "S2 Ep. 1");
    expect(exportTitle([{ manga: tog, chapters: [s1] }])).toBe("Tower of God S1 Ep. 1");
    expect(exportTitle([{ manga: tog, chapters: [s2, chapter("80", "S2 Ep. 0"), s1] }])).toBe(
      "Tower of God S1 Ep. 1 - S2 Ep. 1 (3 ch)",
    );
    expect(exportTitle([{ manga: tog, chapters: [chapter("1", "Notice"), s2] }])).toBe(
      "Tower of God ch 1 - S2 Ep. 1 (2 ch)",
    );
    expect(exportTitle([{ manga: tog, chapters: [chapter("3", "Season finale")] }])).toBe(
      "Tower of God ch 3",
    );
    expect(exportTitle([{ manga: tog, chapters: [chapter("79", "S2 Prologue")] }])).toBe(
      "Tower of God S2 Prologue",
    );
  });
  it("counts extra manga and ignores empty picks", () => {
    expect(
      exportTitle([
        { manga: manga("A"), chapters: [] },
        { manga: manga("B"), chapters: [chapter("1")] },
        { manga: manga("C"), chapters: [chapter("1")] },
      ]),
    ).toBe("B +1 more");
    expect(exportTitle([])).toBe("Manga");
  });
});

describe("paths", () => {
  it("builds a safe file name with the format extension inside the chosen folder", async () => {
    expect(await exportFileName('Re:Zero / "Arc 4"?')).toBe("Re Zero Arc 4.epub");
    expect(await exportFileName("Solo ch 1", "cbz")).toBe("Solo ch 1.cbz");
    expect(await exportFileName("Solo ch 1", "pdf")).toBe("Solo ch 1.pdf");
    const long = await exportFileName("進".repeat(100), "pdf");
    expect(long).toBe(`${"進".repeat(65)}….pdf`);
    expect(joinPath("/out/", "a.epub")).toBe("/out/a.epub");
    expect(joinPath("C:\\Users\\m\\Kindle", "a.epub")).toBe("C:\\Users\\m\\Kindle/a.epub");
    expect(parentDir("/out/a.epub")).toBe("/out");
    expect(parentDir("a.epub")).toBe("a.epub");
    expect(parentDir("/a.epub")).toBe("/");
  });
});
