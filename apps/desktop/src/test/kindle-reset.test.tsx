import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { type Backend, setBackend } from "@/lib/ipc";
import { EMPTY_EMAIL } from "@/lib/ipc.send";
import { loadSettings, writeSettings } from "@/lib/settings";
import { GROUPS } from "@/screens/settings-groups";

const toast = vi.hoisted(() => ({ success: vi.fn(), warning: vi.fn(), error: vi.fn() }));
vi.mock("sonner", () => ({ toast }));

const Kindle = GROUPS.find((g) => g.id === "kindle")?.Body ?? (() => null);
const status = { has_password: true, storage: "keychain", fallback: null };
const passwordInput = () => screen.getByLabelText("Password") as HTMLInputElement;

function renderKindle(setSmtpPassword: () => Promise<unknown>) {
  const smtpPasswordStatus = vi.fn(async () => status);
  setBackend({ smtpPasswordStatus, setSmtpPassword, setSettings: vi.fn() } as unknown as Backend);
  writeSettings({ kindleEmail: { ...EMPTY_EMAIL, username: "me", has_password: true } });
  render(<Kindle />);
}

async function confirmReset() {
  fireEvent.click(screen.getByRole("button", { name: "Reset to defaults" }));
  fireEvent.click(await screen.findByRole("button", { name: "Reset" }));
}

afterEach(() => {
  // Back to a load in flight: a failed reload in one test (these backends have no
  // get_settings) would otherwise make the next test's edits a no-op.
  setBackend({ getSettings: () => new Promise(() => {}) } as unknown as Backend);
  void loadSettings();
  writeSettings({ kindleEmail: EMPTY_EMAIL });
  vi.clearAllMocks();
});

describe("Kindle settings reset", () => {
  it("drops a typed, unsaved password along with the saved one", async () => {
    const setSmtpPassword = vi.fn(async () => ({ ...status, has_password: false }));
    renderKindle(setSmtpPassword);
    fireEvent.click(await screen.findByRole("button", { name: "Change" }));
    fireEvent.change(passwordInput(), { target: { value: "new-secret" } });
    await confirmReset();
    await waitFor(() =>
      expect(toast.success).toHaveBeenCalledWith("Kindle settings reset", {
        id: "settings-reset-Kindle",
        description: null,
        action: null,
      }),
    );
    expect(setSmtpPassword).toHaveBeenCalledWith("");
    expect(passwordInput().value).toBe("");
    expect((screen.getByRole("button", { name: "Save" }) as HTMLButtonElement).disabled).toBe(true);
  });

  it("doesn't report the reset as done when the password couldn't be cleared", async () => {
    renderKindle(vi.fn(async () => Promise.reject("The system keychain is locked")));
    await screen.findByRole("button", { name: "Change" });
    await confirmReset();
    await waitFor(() => expect(toast.error).toHaveBeenCalled());
    expect(toast.success).not.toHaveBeenCalled();
  });
});
