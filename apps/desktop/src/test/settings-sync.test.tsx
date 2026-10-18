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

  it("sends only the changed field and one write at a time", async () => {
    const loaded: Settings = {
      ...(await mockBackend.getSettings()),
      update_check_hours: null,
    };
    let release = () => {};
    const first = new Promise<void>((r) => {
      release = r;
    });
    const firstAnswer: Settings = { ...loaded, default_device: "kindle-scribe" };
    const setSettings = vi
      .fn()
      .mockImplementationOnce(async () => {
        await first;
        return firstAnswer;
      })
      .mockImplementationOnce(async (edited: Settings) => edited);
    const getSettings = async () => loaded;
    const defaultDataDir = async () => "/home/me/.local/share/mangas";
    setBackend({ getSettings, setSettings, defaultDataDir } as unknown as Backend);
    await loadSettings();
    writeSettings({ defaultDevice: "kindle-scribe" });
    writeSettings({ updateNotifications: false });
    await waitFor(() => expect(setSettings).toHaveBeenCalledTimes(1));
    expect(setSettings.mock.calls[0][0].update_check_hours).toBeNull();
    release();
    await waitFor(() => expect(setSettings).toHaveBeenCalledTimes(2));
    const [edited, base] = setSettings.mock.calls[1];
    expect(base).toBe(firstAnswer);
    expect(edited).toEqual({ ...firstAnswer, update_notifications: false });
    await waitFor(() => expect(readSettings().updateNotifications).toBe(false));
  });

  it("a save after a CLI storage move keeps the new data folder", async () => {
    const loaded: Settings = { ...(await mockBackend.getSettings()), data_dir: "/old" };
    const afterMove: Settings = { ...loaded, data_dir: "/mnt/big-disk" };
    const setSettings = vi.fn(async (edited: Settings, base?: Settings) => ({
      ...afterMove,
      ...(edited.default_device !== base?.default_device && {
        default_device: edited.default_device,
      }),
    }));
    const getSettings = async () => loaded;
    const defaultDataDir = async () => "/home/me/.local/share/mangas";
    setBackend({ getSettings, setSettings, defaultDataDir } as unknown as Backend);
    await loadSettings();
    writeSettings({ defaultDevice: "kindle-oasis" });
    await waitFor(() => expect(readSettings().dataDir).toBe("/mnt/big-disk"));
    const [edited, base] = setSettings.mock.calls[0];
    expect(edited.data_dir).toBe(base?.data_dir);
    expect(readSettings().defaultDevice).toBe("kindle-oasis");
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
    await waitFor(() =>
      expect((screen.getByRole("button", { name: "Import" }) as HTMLButtonElement).disabled).toBe(
        false,
      ),
    );
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
    await waitFor(() =>
      expect((screen.getByRole("button", { name: "Import" }) as HTMLButtonElement).disabled).toBe(
        false,
      ),
    );
    fireEvent.click(screen.getByRole("button", { name: "Import" }));
    await waitFor(() =>
      expect(importBackup).toHaveBeenCalledWith("~/mangas-backup.json", "backup"),
    );
  });
});
