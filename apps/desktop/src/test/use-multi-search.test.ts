import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import { useMultiSearch } from "@/hooks/use-multi-search";
import { type Backend, type SearchGroup, setBackend } from "@/lib/ipc";
import { cachedResults, forgetSearch, rememberResults } from "@/lib/search-session";
import { loadSettings, writeSettings } from "@/lib/settings";

const answer = (provider: string): SearchGroup => ({
  provider,
  status: { state: "ready" },
  items: [],
});

function backend() {
  const searchAll = vi.fn(async (_q: string, only: string[]) => only.map(answer));
  setBackend({ searchAll } as unknown as Backend);
  return searchAll;
}

/** The hook waits for the first settings load (#99); this file isn't about that. */
beforeAll(async () => {
  setBackend({
    getSettings: async () => ({ provider_sessions: {} }),
    defaultDataDir: async () => "/data",
  } as unknown as Backend);
  await loadSettings();
  setBackend(null);
});

afterEach(() => {
  setBackend(null);
  forgetSearch();
});

describe("search session", () => {
  it("keeps results per query and provider list, never empty ones", () => {
    rememberResults("dandadan", "mangadex", {});
    expect(cachedResults("dandadan", "mangadex")).toBeNull();
    const groups = { mangadex: { kind: "searching" as const } };
    rememberResults("dandadan", "mangadex", groups);
    expect(cachedResults("dandadan", "mangadex")).toBe(groups);
    expect(cachedResults("dandadan", "mangadex,tapas")).toBeNull();
    expect(cachedResults("berserk", "mangadex")).toBeNull();
  });
});

describe("useMultiSearch", () => {
  it("shows the last results again on remount without searching again", async () => {
    const searchAll = backend();
    const first = renderHook(() => useMultiSearch("dandadan", ["mangadex", "tapas"]));
    await waitFor(() => expect(first.result.current.groups.tapas?.kind).toBe("done"));
    first.unmount();

    const again = renderHook(() => useMultiSearch("dandadan", ["mangadex", "tapas"]));
    await waitFor(() => expect(again.result.current.groups.mangadex?.kind).toBe("done"));
    expect(again.result.current.groups.tapas?.kind).toBe("done");
    expect(searchAll).toHaveBeenCalledTimes(2);
  });

  it("restores once the provider list loads after the remount", async () => {
    const searchAll = backend();
    const first = renderHook(() => useMultiSearch("dandadan", ["mangadex"]));
    await waitFor(() => expect(first.result.current.groups.mangadex?.kind).toBe("done"));
    first.unmount();

    const again = renderHook(({ ids }) => useMultiSearch("dandadan", ids), {
      initialProps: { ids: [] as string[] },
    });
    again.rerender({ ids: ["mangadex"] });
    await waitFor(() => expect(again.result.current.groups.mangadex?.kind).toBe("done"));
    expect(searchAll).toHaveBeenCalledOnce();
  });

  it("searches again for a different query", async () => {
    const searchAll = backend();
    const { result, rerender } = renderHook(({ q }) => useMultiSearch(q, ["mangadex"]), {
      initialProps: { q: "dandadan" },
    });
    await waitFor(() => expect(result.current.groups.mangadex?.kind).toBe("done"));
    rerender({ q: "berserk" });
    await waitFor(() => expect(searchAll).toHaveBeenCalledTimes(2));
    expect(searchAll).toHaveBeenLastCalledWith("berserk", ["mangadex"]);
  });

  it("searches again after the chapter languages change", async () => {
    const searchAll = backend();
    const first = renderHook(() => useMultiSearch("dandadan", ["mangadex"]));
    await waitFor(() => expect(first.result.current.groups.mangadex?.kind).toBe("done"));
    first.unmount();
    act(() => writeSettings({ preferredLanguages: ["es"] }));

    const again = renderHook(() => useMultiSearch("dandadan", ["mangadex"]));
    await waitFor(() => expect(searchAll).toHaveBeenCalledTimes(2));
    await waitFor(() => expect(again.result.current.groups.mangadex?.kind).toBe("done"));
  });
});
