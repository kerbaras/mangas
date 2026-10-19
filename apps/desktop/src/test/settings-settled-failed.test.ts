import { renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { useMangaDetails } from "@/hooks/use-manga-details";
import { useMultiSearch } from "@/hooks/use-multi-search";
import { type Backend, type Manga, type Settings, setBackend } from "@/lib/ipc";
import { loadSettings, readSettings } from "@/lib/settings";

/** Its own file: the first settings load of this module has to fail (#99). */
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

describe("a failed first settings load (#99)", () => {
  it("releases Details and Search, and the retry that loads fetches them again", async () => {
    let answers = 0;
    const calls: string[] = [];
    const langs = () => readSettings().preferredLanguages.join();
    setBackend({
      getSettings: async () => {
        answers++;
        if (answers === 1) throw new Error("database is locked");
        return stored;
      },
      defaultDataDir: async () => "/data",
      getManga: async () => {
        calls.push(`details in ${langs()}`);
        return { ...card, description: langs() };
      },
      searchAll: async (_q: string, only: string[]) => {
        calls.push(`search in ${langs()}`);
        return only.map((provider) => ({ provider, status: { state: "ready" }, items: [] }));
      },
    } as unknown as Backend);
    const details = renderHook(() => useMangaDetails(card));
    renderHook(() => useMultiSearch("dandadan", ["mangadex"]));

    await loadSettings();
    await waitFor(() => expect(calls).toHaveLength(2));
    const placeholder = langs();
    expect(placeholder).not.toBe("es-la");
    expect(details.result.current.manga.description).toBe(placeholder);

    await loadSettings();
    await waitFor(() => expect(calls).toHaveLength(4));
    expect(calls.slice(2).sort()).toEqual(["details in es-la", "search in es-la"]);
    await waitFor(() => expect(details.result.current.manga.description).toBe("es-la"));

    // Once the stored settings are in, reloading them (window focus) fetches nothing again.
    await loadSettings();
    await new Promise((r) => setTimeout(r, 20));
    expect(calls).toHaveLength(4);
    setBackend(null);
  });
});
