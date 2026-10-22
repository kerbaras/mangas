import { describe, expect, it } from "vitest";
import type { Chapter } from "@/lib/ipc";
import type { BookBudget } from "@/lib/ipc.volumes";
import { againstBudget, alignBudgets, bookDirs, budgetNote, megabytes } from "@/lib/kindle-size";

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
const MB = 1_000_000;
const book = (mb: number, over: boolean, levers: BookBudget["levers"] = null): BookBudget => ({
  pages: 304,
  estimated_bytes: mb * MB,
  budget_bytes: 200 * MB,
  over_budget: over,
  jpeg_quality: 90,
  files: 1,
  levers,
});

describe("kindle size", () => {
  it("formats decimal megabytes like the 50 MB limit", () => {
    expect(megabytes(96_400_000)).toBe("96 MB");
    expect(megabytes(4_250_000)).toBe("4.3 MB");
  });

  it("sizes the books with downloaded chapters and puts them back in plan order", () => {
    const one = plan([chapter("1", true), chapter("2", false)]);
    const none = plan([chapter("6", false)]);
    const two = plan([chapter("4", true), chapter("5", true)]);
    const dirs = bookDirs([one, none, two]);
    expect(dirs).toEqual([["/d/1"], [], ["/d/4", "/d/5"]]);
    const [a, b] = [book(10, false), book(20, false)];
    expect(alignBudgets(dirs, [a, b])).toEqual([a, null, b]);
  });

  it("shows the largest book against its budget with the quality, and core's over flag", () => {
    expect(againstBudget(book(212, true))).toBe("212 MB of 200 MB");
    const note = budgetNote([book(120, false), book(212, true)]);
    expect(note.label).toBe("Largest book: about 212 MB of 200 MB, 304 pages, JPEG quality 90");
    expect(note).toMatchObject({ over: 1, budgetMb: 200 });
    expect(budgetNote([{ ...book(212, false), files: 2 }]).label).toBe(
      "About 212 MB in 2 parts of 200 MB, 304 pages, JPEG quality 90",
    );
  });

  it("offers the levers that fit every over-budget book", () => {
    const l = (chapters: number | null, quality: number | null) => ({
      split_bytes: 180 * MB,
      chapters_per_book: chapters,
      jpeg_quality: quality,
    });
    const note = budgetNote([book(250, true, l(8, 80)), book(400, true, l(4, 60))]);
    expect(note.levers).toEqual(l(4, 60));
    expect(budgetNote([book(250, true, l(8, null)), book(400, true, l(4, 60))]).levers).toEqual(
      l(4, null),
    );
    expect(budgetNote([book(100, false)]).levers).toBeNull();
  });
});
