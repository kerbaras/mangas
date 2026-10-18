import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { MangaDexOptionsForm } from "@/components/mangadex-options";
import { type Backend, type Settings, setBackend } from "@/lib/ipc";
import { mockBackend } from "@/lib/ipc.mock";
import { loadSettings } from "@/lib/settings";

afterEach(() => setBackend(null));

describe("MangaDex options", () => {
  it("sends only the changed option, not parser defaults", async () => {
    const loaded: Settings = {
      ...(await mockBackend.getSettings()),
      provider_options: { mangadex: { data_saver: true } },
    };
    const setSettings = vi.fn(async (edited: Settings) => edited);
    const getSettings = async () => loaded;
    const defaultDataDir = async () => "/home/me/.local/share/mangas";
    setBackend({ getSettings, setSettings, defaultDataDir } as unknown as Backend);
    await loadSettings();
    render(<MangaDexOptionsForm />);
    fireEvent.click(screen.getByRole("checkbox", { name: "Erotica" }));
    await waitFor(() => expect(setSettings).toHaveBeenCalledTimes(1));
    const [edited, base] = setSettings.mock.calls[0] as unknown as [Settings, Settings];
    expect(base.provider_options.mangadex).toEqual({ data_saver: true });
    expect(edited.provider_options.mangadex).toEqual({
      data_saver: true,
      content_ratings: ["safe", "suggestive", "erotica"],
    });
  });
});
