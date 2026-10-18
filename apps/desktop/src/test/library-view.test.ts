import { describe, expect, it } from "vitest";
import type { LibraryEntry } from "@/lib/ipc.library";
import {
  DEFAULT_VIEW,
  filterEntries,
  liveView,
  parseView,
  sortEntries,
  sourcesOf,
} from "@/lib/library-view";

const entry = (id: string, over: Partial<LibraryEntry> = {}): LibraryEntry => ({
  id,
  source: "mangadex",
  title: id,
  description: "",
  cover_url: null,
  status: "ongoing",
  reading_status: "reading",
  categories: [],
  added_at: "2026-01-01T00:00:00Z",
  updated_at: "2026-01-01T00:00:00Z",
  chapters: { total: 10, downloaded: 0, exported: 0, unread: 0 },
  ...over,
});

const all = [
  entry("Berserk", { categories: [1, 2], updated_at: "2026-03-01T00:00:00Z" }),
  entry("Vagabond", {
    source: "weebcentral",
    reading_status: "on_hold",
    added_at: "2026-02-01T00:00:00Z",
    chapters: { total: 10, downloaded: 4, exported: 1, unread: 9 },
  }),
  entry("Akira", {
    categories: [2],
    chapters: { total: 3, downloaded: 3, exported: 0, unread: 3 },
  }),
];
const titles = (list: LibraryEntry[]) => list.map((e) => e.title);

describe("library view", () => {
  it("filters by category, source, status and title together", () => {
    expect(titles(filterEntries(all, { ...DEFAULT_VIEW, category: 2 }, ""))).toEqual([
      "Berserk",
      "Akira",
    ]);
    expect(titles(filterEntries(all, { ...DEFAULT_VIEW, source: "weebcentral" }, ""))).toEqual([
      "Vagabond",
    ]);
    expect(titles(filterEntries(all, { ...DEFAULT_VIEW, status: "reading" }, "ak"))).toEqual([
      "Akira",
    ]);
    expect(filterEntries(all, { ...DEFAULT_VIEW, category: 1, status: "on_hold" }, "")).toEqual([]);
  });

  it("sorts by title, newest update, most unread and newest follow", () => {
    expect(titles(sortEntries(all, "title"))).toEqual(["Akira", "Berserk", "Vagabond"]);
    expect(titles(sortEntries(all, "updated"))).toEqual(["Berserk", "Akira", "Vagabond"]);
    expect(titles(sortEntries(all, "unread"))).toEqual(["Vagabond", "Akira", "Berserk"]);
    expect(titles(sortEntries(all, "added"))).toEqual(["Vagabond", "Akira", "Berserk"]);
  });

  it("reads a stored view field by field and ignores junk", () => {
    expect(parseView(null)).toEqual(DEFAULT_VIEW);
    expect(parseView("{oops")).toEqual(DEFAULT_VIEW);
    expect(
      parseView(JSON.stringify({ category: 2, status: "dropped", sort: "unread", layout: "list" })),
    ).toEqual({ ...DEFAULT_VIEW, category: 2, status: "dropped", sort: "unread", layout: "list" });
    expect(parseView(JSON.stringify({ status: "binge", sort: 3, layout: "table" }))).toEqual(
      DEFAULT_VIEW,
    );
  });

  it("drops a remembered category or source that is gone", () => {
    expect(sourcesOf(all)).toEqual(["mangadex", "weebcentral"]);
    const stale = { ...DEFAULT_VIEW, category: 9, source: "tapas", status: "dropped" as const };
    expect(liveView(stale, all, [1, 2])).toEqual({ ...stale, category: null, source: null });
  });
});
