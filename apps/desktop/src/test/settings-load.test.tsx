import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { toast } from "sonner";
import { afterEach, describe, expect, it, vi } from "vitest";
import { SettingsGroup } from "@/components/settings-group";
import { type Backend, setBackend } from "@/lib/ipc";
import { mockBackend } from "@/lib/ipc.mock";
import { notifyError } from "@/lib/notify";
import {
  loadSettings,
  readSettings,
  readSettingsLoad,
  saveSettings,
  writeSettings,
} from "@/lib/settings";
import { SettingsScreen } from "@/screens/settings";

vi.mock("@/lib/notify", async (actual) => ({
  ...(await actual<typeof import("@/lib/notify")>()),
  notifyError: vi.fn(),
}));
vi.mock("sonner", async (actual) => {
  const real = await actual<typeof import("sonner")>();
  return { ...real, toast: Object.assign(real.toast, { success: vi.fn() }) };
});

function backend(getSettings: Backend["getSettings"]) {
  const setSettings = vi.fn(async () => {});
  const defaultDataDir = async () => "/home/me/.local/share/mangas";
  setBackend({ getSettings, setSettings, defaultDataDir } as unknown as Backend);
  return setSettings;
}

afterEach(() => setBackend(null));

describe("settings load", () => {
  it("keeps a failed load instead of throwing, and does not save over the stored settings", async () => {
    const setSettings = backend(async () => {
      throw "library: database is locked";
    });
    await loadSettings();
    expect(readSettingsLoad()).toEqual({ kind: "failed", error: "library: database is locked" });
    writeSettings({ defaultDevice: "kindle-scribe" });
    expect(setSettings).not.toHaveBeenCalled();
  });

  it("shows the error on the Settings screen and retries from it", async () => {
    const getSettings = vi.fn(async () => {
      throw "library: database is locked";
    });
    backend(getSettings);
    await loadSettings();
    render(<SettingsScreen />);
    expect(screen.getByText("Couldn't load your settings")).toBeTruthy();
    expect(screen.getByText("library: database is locked")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: /try again/i }));
    await waitFor(() => expect(getSettings).toHaveBeenCalledTimes(2));
  });

  it("still shows the groups on the defaults, and Save refuses with why", async () => {
    const setSettings = backend(async () => {
      throw "settings: key must be a string at line 1 column 2";
    });
    await loadSettings();
    render(<SettingsScreen />);
    expect(screen.getByRole("navigation", { name: "Settings groups" })).toBeTruthy();
    expect(screen.getByText(/Changes aren't saved until your settings load/)).toBeTruthy();
    await expect(saveSettings({ defaultDevice: "kindle-scribe" })).rejects.toThrow(
      "settings: key must be a string at line 1 column 2",
    );
    expect(setSettings).not.toHaveBeenCalled();
  });

  it("changes nothing after a failed load, and Reset to defaults says it didn't save", async () => {
    const error = "settings: key must be a string at line 1 column 2";
    const setSettings = backend(async () => {
      throw error;
    });
    await loadSettings();
    const before = readSettings();
    expect(await writeSettings({ defaultDevice: "kindle-scribe" })).toBe(false);
    expect(readSettings()).toBe(before);
    render(
      <SettingsGroup title="Export" resets={["defaultDevice"]}>
        <span />
      </SettingsGroup>,
    );
    fireEvent.click(screen.getByRole("button", { name: /reset to defaults/i }));
    fireEvent.click(screen.getByRole("button", { name: "Reset" }));
    await waitFor(() =>
      expect(notifyError).toHaveBeenCalledWith("Couldn't reset export settings", error, {
        id: "settings-reset-Export",
      }),
    );
    expect(toast.success).not.toHaveBeenCalled();
    expect(readSettings()).toBe(before);
    expect(setSettings).not.toHaveBeenCalled();
  });

  it("keeps settings that loaded when an overlapping load fails later", async () => {
    const stored = await mockBackend.getSettings();
    let calls = 0;
    backend(async () => {
      calls += 1;
      if (calls === 1) return stored;
      await new Promise((r) => setTimeout(r, 10));
      throw "library: database is locked";
    });
    await Promise.all([loadSettings(), loadSettings()]);
    expect(readSettingsLoad()).toEqual({ kind: "ready" });
  });

  it("never lets an older answer replace a newer one", async () => {
    const stored = await mockBackend.getSettings();
    let calls = 0;
    backend(async () => {
      calls += 1;
      if (calls === 2) return { ...stored, default_device: "kindle-scribe" };
      await new Promise((r) => setTimeout(r, 10));
      return { ...stored, default_device: "kindle-oasis" };
    });
    await Promise.all([loadSettings(), loadSettings()]);
    expect(readSettings().defaultDevice).toBe("kindle-scribe");
  });
});
