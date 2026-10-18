import { describe, expect, it } from "vitest";
import { exportFileName, exportTitle, joinPath, parentDir } from "@/lib/export-name";
import type { Chapter, Manga } from "@/lib/ipc";

const manga = (title: string): Manga => ({
  id: title.toLowerCase(),
  source: "mangadex",
  title,
  description: "",
  cover_url: null,
  status: "ongoing",
});
const chapter = (number: string): Chapter => ({
  id: `c${number}`,
  manga_id: "m",
  title: null,
  language: "en",
  volume: null,
  number,
  local_path: `/dl/${number}`,
});

describe("exportTitle", () => {
  it("uses the chapter range of a single manga, sorted numerically", () => {
    const picks = [{ manga: manga("Solo Leveling"), chapters: ["10", "2", "3"].map(chapter) }];
    expect(exportTitle(picks)).toBe("Solo Leveling ch 2-10 (3 ch)");
    expect(exportTitle([{ manga: manga("ORV"), chapters: [chapter("262")] }])).toBe("ORV ch 262");
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
  it("builds a safe file name with the format extension inside the chosen folder", () => {
    expect(exportFileName('Re:Zero / "Arc 4"?')).toBe("Re Zero Arc 4.epub");
    expect(exportFileName("Solo ch 1", "cbz")).toBe("Solo ch 1.cbz");
    expect(exportFileName("Solo ch 1", "pdf")).toBe("Solo ch 1.pdf");
    expect(joinPath("/out/", "a.epub")).toBe("/out/a.epub");
    expect(joinPath("C:\\Users\\m\\Kindle", "a.epub")).toBe("C:\\Users\\m\\Kindle/a.epub");
    expect(parentDir("/out/a.epub")).toBe("/out");
    expect(parentDir("a.epub")).toBe("a.epub");
    expect(parentDir("/a.epub")).toBe("/");
  });
});
