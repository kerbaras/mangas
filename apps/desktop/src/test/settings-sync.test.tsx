import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { Toaster } from "sonner";
import { afterEach, describe, expect, it, vi } from "vitest";
import { BackupPanel } from "@/components/backup-panel";
import { onAppFocus, watchAppFocus } from "@/lib/app-focus";
import { type Backend, type Settings, setBackend } from "@/lib/ipc";
import { mockBackend } from "@/lib/ipc.mock";
import { loadSettings, readSettings, writeSettings } from "@/lib/settings";

afterEach(() => setBackend(null));

describe("settings changed by another process", () => {
  it("sends the copy it edited and adopts what the backend stored", async () => {
    const loaded = await mockBackend.getSettings();
    const fromCli: Settings = { ...loaded, preferred_languages: ["es"] };
    const setSettings = vi.fn(async (edited: Settings) => ({
      ...fromCli,
      default_device: edited.default_device,
    }));
    const getSettings = async () => loaded;
    const defaultDataDir = async () => "/home/me/.local/share/mangas";
    setBackend({ getSettings, setSettings, defaultDataDir } as unknown as Backend);
    await loadSettings();
    writeSettings({ defaultDevice: "kindle-scribe" });
    await waitFor(() => expect(readSettings().preferredLanguages).toEqual(["es"]));
    expect(setSettings).toHaveBeenCalledWith(
      expect.objectContaining({ default_device: "kindle-scribe" }),
      loaded,
    );
    expect(readSettings().defaultDevice).toBe("kindle-scribe");
  });

  it("tells listeners once when the window comes back to the front", () => {
    const stop = watchAppFocus();
    const listener = vi.fn();
    const off = onAppFocus(listener);
    act(() => {
      window.dispatchEvent(new Event("focus"));
      document.dispatchEvent(new Event("visibilitychange"));
    });
    expect(listener).toHaveBeenCalledTimes(1);
    off();
    stop();
  });
});

describe("restore dialog", () => {
  function backend(titles: number) {
    const importBackup = vi.fn(async () => ({
      added: 1,
      already: 0,
      skipped: [],
      settings: false,
      kept_settings: true,
    }));
    const listLibrary = async () => Array.from({ length: titles }, (_, i) => ({ id: `${i}` }));
    setBackend({
      ...mockBackend,
      importBackup,
      listLibrary,
    } as unknown as Backend);
    return importBackup;
  }

  const open = async () => {
    render(
      <>
        <BackupPanel />
        <Toaster />
      </>,
    );
    fireEvent.click(screen.getByRole("button", { name: /import backup/i }));
    return (await screen.findByLabelText(/keep my current preferences/i)) as HTMLInputElement;
  };

  it("keeps your preferences by default when the library has titles", async () => {
    const importBackup = backend(2);
    const keep = await open();
    await waitFor(() => expect(keep.checked).toBe(true));
    fireEvent.click(screen.getByRole("button", { name: "Import" }));
    await waitFor(() => expect(importBackup).toHaveBeenCalledWith("~/mangas-backup.json", "keep"));
    expect(await screen.findByText(/your preferences were kept/i)).toBeTruthy();
  });

  it("takes the backup's preferences into an empty library, or when unticked", async () => {
    const importBackup = backend(0);
    const keep = await open();
    await waitFor(() => expect(keep.checked).toBe(false));
    fireEvent.click(keep);
    fireEvent.click(keep);
    fireEvent.click(screen.getByRole("button", { name: "Import" }));
    await waitFor(() =>
      expect(importBackup).toHaveBeenCalledWith("~/mangas-backup.json", "backup"),
    );
  });
});
