import { afterEach, describe, expect, it, vi } from "vitest";
import { type Backend, type Chapter, setBackend } from "@/lib/ipc";
import { loadChapters } from "@/lib/load-chapters";

vi.mock("@/lib/settings", () => ({ settingsSettled: () => Promise.resolve() }));

const stored: Chapter = {
  id: "c1",
  manga_id: "m1",
  title: null,
  language: "en",
  volume: null,
  number: "1",
  local_path: null,
};

function backendWith(listChapters: Backend["listChapters"]) {
  setBackend({ listChapters } as Backend);
}

afterEach(() => setBackend(null));

describe("loadChapters", () => {
  it("returns the refreshed list when the source answers", async () => {
    backendWith(async () => [stored]);
    const onError = vi.fn();
    expect(await loadChapters("mangadex", "m1", onError)).toEqual([stored]);
    expect(onError).not.toHaveBeenCalled();
  });
  it("falls back to the stored list and reports the refresh error", async () => {
    backendWith(async (_s, _id, refresh) => {
      if (refresh) throw new Error("network: down");
      return [stored];
    });
    const onError = vi.fn();
    expect(await loadChapters("mangadex", "m1", onError)).toEqual([stored]);
    expect(onError).toHaveBeenCalledWith(new Error("network: down"));
  });
  it("rethrows when nothing is stored", async () => {
    backendWith(async (_s, _id, refresh) => {
      if (refresh) throw new Error("network: down");
      return [];
    });
    await expect(loadChapters("mangadex", "m1", vi.fn())).rejects.toThrow("network: down");
  });
});
