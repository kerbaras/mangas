import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useChapterGroup } from "@/hooks/use-chapter-group";
import type { LibraryEntry } from "@/lib/ipc.library";

const calls: (string | null)[] = [];
const resolvers: (() => void)[] = [];
vi.mock("@/lib/ipc.updates", () => ({
  setChapterGroup: (_s: string, _id: string, group: string | null) => {
    calls.push(group);
    return new Promise<boolean>((done) => resolvers.push(() => done(true)));
  },
}));

const entry = (chapter_group: string | null) =>
  ({ source: "mangadot", id: "2098", chapter_group }) as LibraryEntry;

beforeEach(() => {
  calls.length = 0;
  resolvers.length = 0;
});

describe("useChapterGroup", () => {
  it("saves rapid picks one after another, last pick last", async () => {
    const onSaved = vi.fn();
    const followed = entry(null);
    const { result } = renderHook(() => useChapterGroup(followed, onSaved));
    act(() => result.current.setGroup("K Manga"));
    act(() => result.current.setGroup("Official"));
    expect(result.current.group).toBe("Official");
    await waitFor(() => expect(calls).toEqual(["K Manga"]));
    await act(async () => resolvers.shift()?.());
    await waitFor(() => expect(calls).toEqual(["K Manga", "Official"]));
  });

  it("saves a group picked before following once the title is followed", async () => {
    const { result, rerender } = renderHook(({ e }) => useChapterGroup(e, () => {}), {
      initialProps: { e: null as LibraryEntry | null },
    });
    act(() => result.current.setGroup("K Manga"));
    expect(calls).toEqual([]);
    rerender({ e: entry(null) });
    await waitFor(() => expect(calls).toEqual(["K Manga"]));
    expect(result.current.group).toBe("K Manga");
  });

  it("clears the group when All is picked while a save is pending", async () => {
    const followed = entry(null);
    const { result } = renderHook(() => useChapterGroup(followed, () => {}));
    act(() => result.current.setGroup("K Manga"));
    act(() => result.current.setGroup(null));
    expect(result.current.group).toBeNull();
    await waitFor(() => expect(calls).toEqual(["K Manga"]));
    await act(async () => resolvers.shift()?.());
    await waitFor(() => expect(calls).toEqual(["K Manga", null]));
  });

  it("follows a group saved elsewhere once nothing is pending", async () => {
    const { result, rerender } = renderHook(({ e }) => useChapterGroup(e, () => {}), {
      initialProps: { e: entry(null) },
    });
    act(() => result.current.setGroup("K Manga"));
    await waitFor(() => expect(calls).toEqual(["K Manga"]));
    await act(async () => resolvers.shift()?.());
    rerender({ e: entry("Official") });
    expect(result.current.group).toBe("Official");
    expect(calls).toEqual(["K Manga"]);
  });

  it("shows a group saved elsewhere during a pending save once it settles", async () => {
    const { result, rerender } = renderHook(({ e }) => useChapterGroup(e, () => {}), {
      initialProps: { e: entry(null) },
    });
    act(() => result.current.setGroup("K Manga"));
    await waitFor(() => expect(calls).toEqual(["K Manga"]));
    rerender({ e: entry("Official") });
    expect(result.current.group).toBe("K Manga");
    await act(async () => resolvers.shift()?.());
    rerender({ e: entry("Official") });
    expect(result.current.group).toBe("Official");
  });

  it("starts from the saved choice", () => {
    const { result } = renderHook(() => useChapterGroup(entry("Official"), () => {}));
    expect(result.current.group).toBe("Official");
    expect(calls).toEqual([]);
  });
});
