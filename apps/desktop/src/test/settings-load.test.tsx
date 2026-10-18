import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { type Backend, setBackend } from "@/lib/ipc";
import { loadSettings, readSettingsLoad, writeSettings } from "@/lib/settings";
import { SettingsScreen } from "@/screens/settings";

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
});
