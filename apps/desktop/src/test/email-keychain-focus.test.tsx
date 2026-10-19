import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { EmailSettingsForm } from "@/components/email-settings";
import { onAppFocus, watchAppFocus } from "@/lib/app-focus";
import { type Backend, setBackend } from "@/lib/ipc";
import { mockBackend } from "@/lib/ipc.mock";
import { loadSettings, readSettings } from "@/lib/settings";

afterEach(() => {
  setBackend(null);
  vi.restoreAllMocks();
});

describe("a locked keychain and window focus", () => {
  it("never asks the keychain on focus; opening the email settings asks once", async () => {
    const loaded = await mockBackend.getSettings();
    const getSettings = vi.fn(async () => loaded);
    const emailPassword = vi.fn(async (_retry: boolean) => ({ password: "", storage: "locked" }));
    const defaultDataDir = async () => "/home/me/.local/share/mangas";
    setBackend({ getSettings, defaultDataDir, emailPassword } as unknown as Backend);
    let now = 0;
    vi.spyOn(Date, "now").mockImplementation(() => (now += 1000));
    const stop = watchAppFocus();
    const off = onAppFocus(() => void loadSettings());
    const focus3 = () => {
      for (let i = 0; i < 3; i++) fireEvent.focus(window);
    };

    focus3();
    await waitFor(() => expect(getSettings).toHaveBeenCalledTimes(3));
    expect(emailPassword).not.toHaveBeenCalled();

    render(<EmailSettingsForm />);
    await screen.findByText("Your system keychain is locked or can't be read.");
    focus3();
    await waitFor(() => expect(getSettings).toHaveBeenCalledTimes(6));
    expect(emailPassword.mock.calls).toEqual([[true]]);
    off();
    stop();
  });

  it("fails a save into a locked keychain, keeping the typed password", async () => {
    const loaded = await mockBackend.getSettings();
    const locked =
      "The system keychain is locked or can't be read, so the SMTP password isn't available. Unlock it and try again (prompt dismissed)";
    const setSettings = vi.fn(async () => {
      throw new Error(locked);
    });
    const emailPassword = vi.fn(async () => ({ password: "", storage: "locked" }));
    const getSettings = async () => loaded;
    const defaultDataDir = async () => "/home/me/.local/share/mangas";
    setBackend({ getSettings, setSettings, defaultDataDir, emailPassword } as unknown as Backend);
    await loadSettings();
    render(<EmailSettingsForm />);
    await screen.findByText("Your system keychain is locked or can't be read.");
    fireEvent.change(screen.getByLabelText("Password"), { target: { value: "new-secret" } });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    expect(await screen.findByText(locked)).toBeTruthy();
    expect(setSettings).toHaveBeenCalledTimes(1);
    expect(readSettings().kindleEmail.password).toBe("");
    expect((screen.getByLabelText("Password") as HTMLInputElement).value).toBe("new-secret");
  });
});
