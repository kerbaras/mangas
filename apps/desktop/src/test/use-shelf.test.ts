import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { useShelf } from "@/hooks/use-shelf";
import { type LibraryEntry, listEntries, setMangaCategories } from "@/lib/ipc.library";

const entry = {
  id: "berserk",
  source: "mangadex",
  title: "Berserk",
  categories: [],
  reading_status: "reading",
  chapters: { total: 1, downloaded: 0, exported: 0, unread: 1 },
} as unknown as LibraryEntry;

vi.mock("@/lib/ipc", () => ({ unfollowManga: vi.fn() }));
vi.mock("@/lib/ipc.library", () => ({
  listEntries: vi.fn(async () => [entry]),
  listCategories: vi.fn(async () => [
    { id: 1, name: "Favourites" },
    { id: 2, name: "Weekly" },
  ]),
  setMangaCategories: vi.fn(async () => {}),
}));

describe("useShelf", () => {
  it("keeps both of two quick category toggles on the same title", async () => {
    const { result } = renderHook(() => useShelf());
    await waitFor(() => expect(result.current.state.kind).toBe("ready"));
    const [stale] = await listEntries();

    await act(() =>
      Promise.all([
        result.current.toggleCategory(stale, 1, true),
        result.current.toggleCategory(stale, 2, true),
      ]),
    );

    expect(vi.mocked(setMangaCategories).mock.calls.at(-1)).toEqual([
      "mangadex",
      "berserk",
      [1, 2],
    ]);
    const state = result.current.state;
    expect(state.kind === "ready" && state.data.entries[0].categories).toEqual([1, 2]);
  });

  it("saves one change after another and reloads when a save fails", async () => {
    const { result } = renderHook(() => useShelf());
    await waitFor(() => expect(result.current.state.kind).toBe("ready"));
    let finishFirst = () => {};
    vi.mocked(setMangaCategories)
      .mockClear()
      .mockImplementationOnce(() => new Promise<void>((r) => (finishFirst = r)))
      .mockRejectedValueOnce("database is locked");
    const loads = vi.mocked(listEntries).mock.calls.length;

    let first: Promise<void> = Promise.resolve();
    let second: Promise<unknown> = Promise.resolve();
    act(() => {
      first = result.current.toggleCategory(entry, 1, true);
      second = result.current.toggleCategory(entry, 2, true).catch((e: unknown) => e);
    });
    await Promise.resolve();
    expect(setMangaCategories).toHaveBeenCalledTimes(1);
    await act(async () => {
      finishFirst();
      await first;
      expect(await second).toBe("database is locked");
    });
    expect(setMangaCategories).toHaveBeenCalledTimes(2);
    await waitFor(() => expect(vi.mocked(listEntries).mock.calls.length).toBeGreaterThan(loads));
  });
});
