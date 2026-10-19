import { describe, expect, it } from "vitest";
import type { Chapter } from "@/lib/ipc";
import { biggestBook, megabytes, sizeNote } from "@/lib/kindle-size";

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

  it("measures the book with the most downloaded chapters", () => {
    const small = plan([chapter("1", true), chapter("2", false), chapter("3", false)]);
    const big = plan([chapter("4", true), chapter("5", true)]);
    expect(biggestBook([small, big])).toEqual(["/d/4", "/d/5"]);
    expect(biggestBook([])).toEqual([]);
  });

  it("warns over 50 MB unless the parts are under it", () => {
    const e = { pages: 183, bytes: 96_000_000 };
    expect(sizeNote(e, 1, null)).toEqual({ label: "About 96 MB, 183 pages", over: true });
    expect(sizeNote(e, 1, 45)).toEqual({
      label: "About 96 MB, 183 pages, about 3 parts",
      over: false,
    });
    expect(sizeNote(e, 1, 60).over).toBe(true);
    expect(sizeNote({ pages: 40, bytes: 20_000_000 }, 3, null)).toEqual({
      label: "Largest book: about 20 MB, 40 pages",
      over: false,
    });
  });
});
