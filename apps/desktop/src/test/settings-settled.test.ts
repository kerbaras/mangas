import { renderHook, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { useMangaDetails } from "@/hooks/use-manga-details";
import { useMultiSearch } from "@/hooks/use-multi-search";
import { type Backend, type Manga, type Settings, setBackend } from "@/lib/ipc";
import { forgetSearch } from "@/lib/search-session";
import { loadSettings, readSettings, writeSettings } from "@/lib/settings";

const card: Manga = {
  id: "m1",
  source: "mangadex",
  title: "Dandadan",
  description: "",
  cover_url: null,
  status: "unknown",
};
const stored: Settings = {
  data_dir: null,
  default_device: null,
  default_preset: null,
  provider_sessions: {},
  preferred_languages: ["es-la"],
};

function deferred<T>() {
  let resolve: (v: T) => void = () => {};
  const promise = new Promise<T>((r) => (resolve = r));
  return { promise, resolve };
}

afterEach(() => {
  setBackend(null);
  forgetSearch();
});

/** Run in order: the first test is the cold start, before any settings loaded. */
describe("fetches wait for the settings (#99)", () => {
  it("on a cold start, Details and Search ask only once the stored settings are in", async () => {
    const settings = deferred<Settings>();
    const calls: string[] = [];
    setBackend({
      getSettings: () => settings.promise,
      defaultDataDir: async () => "/data",
      getManga: async () => {
        calls.push(`details in ${readSettings().preferredLanguages}`);
        return card;
      },
      searchAll: async (_q: string, only: string[]) => {
        calls.push(`search in ${readSettings().preferredLanguages}`);
        return only.map((provider) => ({ provider, status: { state: "ready" }, items: [] }));
      },
    } as unknown as Backend);
    // Screens mount and run their effects before the app's effect starts loading the settings.
    renderHook(() => useMangaDetails(card));
    renderHook(() => useMultiSearch("dandadan", ["mangadex"]));
    const load = loadSettings();
    await new Promise((r) => setTimeout(r, 20));
    expect(calls).toEqual([]);

    settings.resolve(stored);
    await load;
    await waitFor(() => expect(calls).toHaveLength(2));
    expect(calls.sort()).toEqual(["details in es-la", "search in es-la"]);
  });

  it("a search started while a save is in flight runs after the save landed", async () => {
    const saved = deferred<Settings>();
    const order: string[] = [];
    const searchAll = vi.fn(async (_q: string, only: string[]) => {
      order.push("search");
      return only.map((provider) => ({ provider, status: { state: "ready" }, items: [] }));
    });
    setBackend({
      setSettings: async () => {
        await saved.promise;
        order.push("saved");
        return { ...stored, preferred_languages: ["fr"] };
      },
      searchAll,
    } as unknown as Backend);
    writeSettings({ preferredLanguages: ["fr"] });
    const { result } = renderHook(() => useMultiSearch("berserk", ["mangadex"]));
    await new Promise((r) => setTimeout(r, 20));
    expect(searchAll).not.toHaveBeenCalled();

    saved.resolve(stored);
    await waitFor(() => expect(result.current.groups.mangadex?.kind).toBe("done"));
    expect(order).toEqual(["saved", "search"]);
  });
});
