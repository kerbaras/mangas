import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { EmailSettingsForm } from "@/components/email-settings";
import { type Backend, setBackend } from "@/lib/ipc";
import { EMPTY_EMAIL } from "@/lib/ipc.send";
import { loadSettings, readSettings, writeSettings } from "@/lib/settings";

afterEach(() => {
  // Back to a load in flight: a failed reload in one test (these backends have no
  // get_settings) would otherwise make the next test's edits a no-op.
  setBackend({ getSettings: () => new Promise(() => {}) } as unknown as Backend);
  void loadSettings();
  writeSettings({ kindleEmail: EMPTY_EMAIL });
});

const WITH_PASSWORD = { ...EMPTY_EMAIL, username: "me", has_password: true };

const keychain = (storage: string, has_password = false) =>
  vi.fn(async (_retry: boolean) => ({ has_password, storage, fallback: null }));

const type = (label: string, value: string) =>
  fireEvent.change(screen.getByLabelText(label), { target: { value } });

const passwordInput = () => screen.getByLabelText("Password") as HTMLInputElement;

describe("email settings", () => {
  it("shows a saved password only as a placeholder, with Change and Clear", async () => {
    setBackend({ smtpPasswordStatus: keychain("keychain", true) } as unknown as Backend);
    writeSettings({ kindleEmail: WITH_PASSWORD });
    render(<EmailSettingsForm />);
    expect(await screen.findByText("Kept in your system keychain.")).toBeTruthy();
    expect(screen.queryByText(/stored unencrypted/)).toBeNull();
    expect(passwordInput().type).toBe("password");
    expect(passwordInput().value).toBe("");
    expect(passwordInput().disabled).toBe(true);
    expect(passwordInput().placeholder).toMatch(/saved/);
    expect(screen.getByRole("button", { name: "Change" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Clear" })).toBeTruthy();
  });

  it("changes the password through set_smtp_password, never the settings", async () => {
    const setSmtpPassword = vi.fn(async () => ({
      has_password: true,
      storage: "keychain",
      fallback: null,
    }));
    const setSettings = vi.fn();
    const smtpPasswordStatus = keychain("keychain", true);
    setBackend({ smtpPasswordStatus, setSmtpPassword, setSettings } as unknown as Backend);
    writeSettings({ kindleEmail: WITH_PASSWORD });
    render(<EmailSettingsForm />);
    fireEvent.click(await screen.findByRole("button", { name: "Change" }));
    expect(passwordInput().disabled).toBe(false);
    type("Password", "new-secret");
    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => expect(setSmtpPassword).toHaveBeenCalledWith("new-secret"));
    expect(setSettings).not.toHaveBeenCalled();
    await waitFor(() => expect(passwordInput().disabled).toBe(true));
  });

  it("clears a saved password", async () => {
    const setSmtpPassword = vi.fn(async () => ({
      has_password: false,
      storage: "keychain",
      fallback: null,
    }));
    setBackend({
      smtpPasswordStatus: keychain("keychain", true),
      setSmtpPassword,
    } as unknown as Backend);
    writeSettings({ kindleEmail: WITH_PASSWORD });
    render(<EmailSettingsForm />);
    fireEvent.click(await screen.findByRole("button", { name: "Clear" }));
    await waitFor(() => expect(setSmtpPassword).toHaveBeenCalledWith(""));
  });

  it("cancels a change and keeps the saved password", async () => {
    setBackend({ smtpPasswordStatus: keychain("keychain", true) } as unknown as Backend);
    writeSettings({ kindleEmail: WITH_PASSWORD });
    render(<EmailSettingsForm />);
    fireEvent.click(await screen.findByRole("button", { name: "Change" }));
    type("Password", "half-typed");
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    expect(passwordInput().value).toBe("");
    expect(passwordInput().disabled).toBe(true);
    expect((screen.getByRole("button", { name: "Save" }) as HTMLButtonElement).disabled).toBe(true);
  });

  it("warns that the password is stored unencrypted without a keychain", async () => {
    setBackend({ smtpPasswordStatus: keychain("file", true) } as unknown as Backend);
    writeSettings({ kindleEmail: WITH_PASSWORD });
    render(<EmailSettingsForm />);
    expect(
      await screen.findByText("No system keychain: the password is stored unencrypted"),
    ).toBeTruthy();
    expect(screen.getByText("Kept in the app settings (no system keychain).")).toBeTruthy();
  });

  it("says the keychain is locked, without the unencrypted warning", async () => {
    setBackend({ smtpPasswordStatus: keychain("locked") } as unknown as Backend);
    render(<EmailSettingsForm />);
    expect(
      await screen.findByText("Your system keychain is locked or can't be read."),
    ).toBeTruthy();
    expect(screen.queryByText(/No password saved/)).toBeNull();
    expect(screen.queryByText(/stored unencrypted/)).toBeNull();
  });

  it("only warns about unencrypted storage when a password sits in the settings", async () => {
    setBackend({ smtpPasswordStatus: keychain("file") } as unknown as Backend);
    render(<EmailSettingsForm />);
    expect(await screen.findByText(/No password saved. Without a system keychain/)).toBeTruthy();
    expect(screen.queryByText(/stored unencrypted/)).toBeNull();
  });

  it("says no password is saved once it is cleared", async () => {
    setBackend({ smtpPasswordStatus: keychain("keychain", true) } as unknown as Backend);
    writeSettings({ kindleEmail: WITH_PASSWORD });
    render(<EmailSettingsForm />);
    expect(await screen.findByText("Kept in your system keychain.")).toBeTruthy();
    act(() => {
      writeSettings({ kindleEmail: EMPTY_EMAIL });
    });
    expect(await screen.findByText("No password saved.")).toBeTruthy();
  });

  it("asks a locked keychain again on a click in the password field", async () => {
    const smtpPasswordStatus = keychain("locked");
    setBackend({ smtpPasswordStatus } as unknown as Backend);
    render(<EmailSettingsForm />);
    await screen.findByText("Your system keychain is locked or can't be read.");
    expect(smtpPasswordStatus.mock.calls).toEqual([[true]]);
    fireEvent.focus(passwordInput());
    await waitFor(() => expect(smtpPasswordStatus.mock.calls).toEqual([[true], [true]]));
  });

  it("tests the unsaved draft and saves it on Save", async () => {
    const sendTestEmail = vi.fn(async () => "me@example.test");
    setBackend({ sendTestEmail } as unknown as Backend);
    render(<EmailSettingsForm />);
    type("Kindle email address", "reader@kindle.com");
    type("SMTP server", "smtp.example.test");
    type("User name", "me@example.test");
    type("Port", "2525");
    fireEvent.click(screen.getByRole("button", { name: /Send test email/ }));
    expect(await screen.findByText("Test email sent to me@example.test.")).toBeTruthy();
    const draft = {
      ...EMPTY_EMAIL,
      to: "reader@kindle.com",
      host: "smtp.example.test",
      username: "me@example.test",
      port: 2525,
    };
    const { has_password: _, ...fields } = draft;
    expect(sendTestEmail).toHaveBeenCalledWith({ ...fields, password: "" }, false);
    expect(readSettings().kindleEmail).toEqual(EMPTY_EMAIL);
    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => expect(readSettings().kindleEmail).toEqual(draft));
  });

  it("tests with a typed password before it is saved", async () => {
    const sendTestEmail = vi.fn(async () => "me@example.test");
    setBackend({ sendTestEmail } as unknown as Backend);
    render(<EmailSettingsForm />);
    type("SMTP server", "smtp.example.test");
    type("Password", "typed-pw");
    fireEvent.click(screen.getByRole("button", { name: /Send test email/ }));
    await screen.findByText(/Test email sent/);
    expect(sendTestEmail.mock.calls[0]).toEqual([
      expect.objectContaining({ password: "typed-pw" }),
      true,
    ]);
  });

  it("keeps unsaved edits when settings reload with the same saved values", async () => {
    render(<EmailSettingsForm />);
    type("SMTP server", "smtp.example.test");
    act(() => {
      writeSettings({ kindleEmail: { ...readSettings().kindleEmail } });
    });
    expect((screen.getByLabelText("SMTP server") as HTMLInputElement).value).toBe(
      "smtp.example.test",
    );
  });

  it("shows a locked keychain on Send test email, asking for the saved password", async () => {
    const locked =
      "The system keychain is locked or can't be read, so the SMTP password isn't available. Unlock it and try again (prompt dismissed)";
    const sendTestEmail = vi.fn(async (_s: unknown, _typed: boolean) => {
      throw new Error(locked);
    });
    setBackend({ sendTestEmail, smtpPasswordStatus: keychain("locked") } as unknown as Backend);
    render(<EmailSettingsForm />);
    type("SMTP server", "smtp.example.test");
    fireEvent.click(screen.getByRole("button", { name: /Send test email/ }));
    expect(await screen.findByText(locked)).toBeTruthy();
    expect(sendTestEmail.mock.calls[0]?.[1]).toBe(false);
  });
});
