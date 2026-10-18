import { afterEach, describe, expect, it, vi } from "vitest";
import { type Backend, type Settings, setBackend } from "@/lib/ipc";
import { mockBackend } from "@/lib/ipc.mock";
import { loadSettings, readSettings, writeSettings } from "@/lib/settings";

afterEach(() => setBackend(null));

const withStored = async (patch: Partial<Settings>) => {
  const loaded: Settings = { ...(await mockBackend.getSettings()), ...patch };
  const setSettings = vi.fn(async (edited: Settings) => edited);
  const backend = {
    getSettings: async () => loaded,
    setSettings,
    defaultDataDir: async () => "/d",
  };
  setBackend(backend as unknown as Backend);
  await loadSettings();
  return setSettings;
};

describe("default export format", () => {
  it("loads a stored format and ignores unknown ones", async () => {
    await withStored({ default_format: "pdf" });
    expect(readSettings().defaultFormat).toBe("pdf");
    await withStored({ default_format: "mobi" as never });
    expect(readSettings().defaultFormat).toBeNull();
  });

  it("persists the picked format as default_format", async () => {
    const setSettings = await withStored({ default_format: null });
    writeSettings({ defaultFormat: "cbz" });
    await vi.waitFor(() =>
      expect(setSettings).toHaveBeenCalledWith(
        expect.objectContaining({ default_format: "cbz" }),
        expect.anything(),
      ),
    );
  });
});
