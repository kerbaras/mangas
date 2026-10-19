import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { RunSetupAgain } from "@/components/run-setup-again";
import { SetupSheet } from "@/components/setup/setup-sheet";
import { type Backend, setBackend } from "@/lib/ipc";
import { mockBackend } from "@/lib/ipc.mock";
import type { Settings as Stored } from "@/lib/ipc.settings";
import { type Navigation, NavigationContext } from "@/lib/navigation";
import { loadSettings, readSettings } from "@/lib/settings";
import { closeSetup } from "@/lib/setup";

async function start(setup_complete: boolean | null) {
  let stored: Stored = { ...(await mockBackend.getSettings()), setup_complete };
  const setSettings = vi.fn(async (next: Stored) => {
    stored = { ...next, provider_sessions: stored.provider_sessions };
    return stored;
  });
  setBackend({ ...mockBackend, getSettings: async () => stored, setSettings } as Backend);
  await loadSettings();
  const go = vi.fn();
  const nav = { route: { screen: "settings" }, go, back: vi.fn(), canGoBack: false } as Navigation;
  render(
    <NavigationContext.Provider value={nav}>
      <RunSetupAgain />
      <SetupSheet />
    </NavigationContext.Provider>,
  );
  return { setSettings, go, stored: () => stored };
}

afterEach(() => {
  closeSetup();
  setBackend(null);
});

describe("first-run setup", () => {
  it("shows on a fresh install; skipping stores the flag and lands in the Library", async () => {
    const { go, stored } = await start(false);
    expect(screen.getByText("Step 1 of 3")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Skip setup" }));
    await waitFor(() => expect(stored().setup_complete).toBe(true));
    expect(go).toHaveBeenCalledWith({ screen: "library" });
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("walks the three steps; Finish keeps the chosen profile and stores the flag", async () => {
    const { go, stored } = await start(false);
    expect(screen.getByLabelText("Data folder")).toHaveProperty("value", readSettings().dataDir);
    fireEvent.click(screen.getByRole("button", { name: "Next" }));
    expect(await screen.findByText("Which e-reader do you use?")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Next" }));
    expect(await screen.findByText("Connect your sources")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Finish" }));
    await waitFor(() => expect(stored().setup_complete).toBe(true));
    expect(go).toHaveBeenCalledWith({ screen: "library" });
  });

  it("stays open with an error when the flag can't be saved", async () => {
    const { setSettings, go } = await start(false);
    setSettings.mockRejectedValueOnce("library: database is locked");
    fireEvent.click(screen.getByRole("button", { name: "Skip setup" }));
    await waitFor(() => expect(setSettings).toHaveBeenCalled());
    await waitFor(() => expect(screen.getByText("Step 1 of 3")).toBeTruthy());
    expect(go).not.toHaveBeenCalled();
  });

  it("keeps retrying the save on Esc when both the save and the reload fail", async () => {
    const { setSettings, go } = await start(false);
    setBackend({
      ...mockBackend,
      getSettings: async () => {
        throw "library: database is locked";
      },
      setSettings,
    } as Backend);
    setSettings.mockRejectedValueOnce("library: database is locked");
    fireEvent.click(screen.getByRole("button", { name: "Skip setup" }));
    await waitFor(() => expect(setSettings).toHaveBeenCalledTimes(1));
    await waitFor(() => expect(screen.getByText("Step 1 of 3")).toBeTruthy());
    fireEvent.keyDown(screen.getByRole("dialog"), { key: "Escape" });
    await waitFor(() => expect(setSettings).toHaveBeenCalledTimes(2));
    await waitFor(() => expect(go).toHaveBeenCalledWith({ screen: "library" }));
  });

  it("never shows on its own once done, or for installs from before it", async () => {
    await start(true);
    expect(screen.queryByRole("dialog")).toBeNull();
    cleanup();
    await start(null);
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("Esc on first run counts as done and lands in the Library", async () => {
    const { go, stored } = await start(false);
    fireEvent.keyDown(screen.getByRole("dialog"), { key: "Escape" });
    await waitFor(() => expect(stored().setup_complete).toBe(true));
    expect(go).toHaveBeenCalledWith({ screen: "library" });
  });

  it("Esc on a sheet reopened from Settings closes it without saving or navigating", async () => {
    const { setSettings, go } = await start(true);
    fireEvent.click(screen.getByRole("button", { name: /run setup again/i }));
    fireEvent.click(screen.getByRole("button", { name: "Next" }));
    fireEvent.keyDown(screen.getByRole("dialog"), { key: "Escape" });
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    expect(setSettings).not.toHaveBeenCalled();
    expect(go).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: /run setup again/i }));
    expect(screen.getByText("Step 1 of 3")).toBeTruthy();
  });

  it("reopens from Settings → Run setup again", async () => {
    await start(true);
    fireEvent.click(screen.getByRole("button", { name: /run setup again/i }));
    expect(screen.getByText("Step 1 of 3")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Next" }));
    fireEvent.click(screen.getByRole("button", { name: "Back" }));
    expect(screen.getByText("Step 1 of 3")).toBeTruthy();
  });
});
