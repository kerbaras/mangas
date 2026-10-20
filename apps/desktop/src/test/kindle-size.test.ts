import { describe, expect, it } from "vitest";
import type { Chapter } from "@/lib/ipc";
import { downloadedBooks, megabytes, sizeNote } from "@/lib/kindle-size";

const chapter = (n: string, local: boolean): Chapter => ({
  id: n,
  manga_id: "m",
  title: null,
  language: "en",
  volume: null,
  number: n,
  local_path: local ? `/d/${n}` : null,
});
const plan = (chapters: Chapter[]) => ({ number: null, title: "t", file_name: "t.epub", chapters });

describe("kindle size", () => {
  it("formats decimal megabytes like the 50 MB limit", () => {
    expect(megabytes(96_400_000)).toBe("96 MB");
    expect(megabytes(4_250_000)).toBe("4.3 MB");
  });

  it("hands every book with downloaded chapters to the estimate", () => {
    const one = plan([chapter("1", true), chapter("2", false), chapter("3", false)]);
    const two = plan([chapter("4", true), chapter("5", true)]);
    const none = plan([chapter("6", false)]);
    expect(downloadedBooks([one, two, none])).toEqual([["/d/1"], ["/d/4", "/d/5"]]);
    expect(downloadedBooks([])).toEqual([]);
  });

  it("warns over 50 MB unless the parts are under it", () => {
    const e = { pages: 183, bytes: 96_000_000 };
    expect(sizeNote(e, 1, null)).toEqual({ label: "About 96 MB, 183 pages", over: true });
    expect(sizeNote(e, 1, 45)).toEqual({
      label: "About 96 MB, 183 pages, 2 or 3 parts",
      over: false,
    });
    expect(sizeNote({ pages: 99, bytes: 70_000_000 }, 1, 45).label).toBe(
      "About 70 MB, 99 pages, about 2 parts",
    );
    expect(sizeNote({ pages: 60, bytes: 42_000_000 }, 1, 45).label).toBe(
      "About 42 MB, 60 pages, 1 or 2 parts",
    );
    expect(sizeNote({ pages: 300, bytes: 200_000_000 }, 1, 45).label).toMatch(/, 4 to 6 parts$/);
    expect(sizeNote({ pages: 4, bytes: 8_000_000 }, 1, 1).label).toMatch(/, about 4 parts$/);
    expect(sizeNote({ pages: 8, bytes: 8_000_000 }, 1, 1).label).toMatch(/, 7 or 8 parts$/);
    expect(sizeNote(e, 1, 60).over).toBe(true);
    expect(sizeNote({ pages: 40, bytes: 20_000_000 }, 3, null)).toEqual({
      label: "Largest book: about 20 MB, 40 pages",
      over: false,
    });
  });
});
