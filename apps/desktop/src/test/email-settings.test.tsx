import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { EmailSettingsForm } from "@/components/email-settings";
import { type Backend, setBackend } from "@/lib/ipc";
import { EMPTY_EMAIL } from "@/lib/ipc.send";
import { readSettings, writeSettings } from "@/lib/settings";

afterEach(() => {
  setBackend(null);
  writeSettings({ kindleEmail: EMPTY_EMAIL });
});

const WITH_PASSWORD = { ...EMPTY_EMAIL, username: "me", password: "app-password" };

const keychain = (storage: string, password = "") =>
  vi.fn(async (_retry: boolean) => ({ password, storage }));

const type = (label: string, value: string) =>
  fireEvent.change(screen.getByLabelText(label), { target: { value } });

describe("email settings", () => {
  it("says the password is in the keychain when there is one", async () => {
    setBackend({ emailPassword: keychain("keychain") } as unknown as Backend);
    writeSettings({ kindleEmail: WITH_PASSWORD });
    render(<EmailSettingsForm />);
    expect(await screen.findByText("Kept in your system keychain.")).toBeTruthy();
    expect(screen.queryByText(/stored unencrypted/)).toBeNull();
    expect((screen.getByLabelText("Password") as HTMLInputElement).type).toBe("password");
  });

  it("warns that the password is stored unencrypted without a keychain", async () => {
    setBackend({ emailPassword: keychain("file") } as unknown as Backend);
    writeSettings({ kindleEmail: WITH_PASSWORD });
    render(<EmailSettingsForm />);
    expect(
      await screen.findByText("No system keychain: the password is stored unencrypted"),
    ).toBeTruthy();
    expect(screen.getByText("Kept in the app settings (no system keychain).")).toBeTruthy();
    expect(screen.queryByText("Kept in your system keychain.")).toBeNull();
  });

  it("says the keychain is locked, without the unencrypted warning", async () => {
    setBackend({ emailPassword: keychain("locked") } as unknown as Backend);
    render(<EmailSettingsForm />);
    expect(
      await screen.findByText("Your system keychain is locked or can't be read."),
    ).toBeTruthy();
    expect(screen.queryByText(/No password saved/)).toBeNull();
    expect(screen.queryByText(/stored unencrypted/)).toBeNull();
  });

  it("only warns about unencrypted storage when a password sits in the settings", async () => {
    setBackend({ emailPassword: keychain("file") } as unknown as Backend);
    render(<EmailSettingsForm />);
    expect(await screen.findByText(/No password saved. Without a system keychain/)).toBeTruthy();
    expect(screen.queryByText(/stored unencrypted/)).toBeNull();
  });

  it("says no password is saved once it is cleared", async () => {
    setBackend({ emailPassword: keychain("keychain") } as unknown as Backend);
    writeSettings({ kindleEmail: WITH_PASSWORD });
    render(<EmailSettingsForm />);
    expect(await screen.findByText("Kept in your system keychain.")).toBeTruthy();
    act(() => writeSettings({ kindleEmail: EMPTY_EMAIL }));
    expect(await screen.findByText("No password saved.")).toBeTruthy();
    expect(screen.queryByText("Kept in your system keychain.")).toBeNull();
  });

  it("shows the password read from the keychain", async () => {
    setBackend({ emailPassword: keychain("keychain", "from-keychain") } as unknown as Backend);
    render(<EmailSettingsForm />);
    expect(await screen.findByText("Kept in your system keychain.")).toBeTruthy();
    expect((screen.getByLabelText("Password") as HTMLInputElement).value).toBe("from-keychain");
  });

  it("asks a locked keychain again on a click in the password field", async () => {
    const emailPassword = keychain("locked");
    setBackend({ emailPassword } as unknown as Backend);
    render(<EmailSettingsForm />);
    await screen.findByText("Your system keychain is locked or can't be read.");
    expect(emailPassword.mock.calls).toEqual([[true]]);
    fireEvent.focus(screen.getByLabelText("Password"));
    await waitFor(() => expect(emailPassword.mock.calls).toEqual([[true], [true]]));
  });

  it("checks where a saved password went without prompting again", async () => {
    const emailPassword = keychain("locked");
    setBackend({ emailPassword } as unknown as Backend);
    render(<EmailSettingsForm />);
    await screen.findByText("Your system keychain is locked or can't be read.");
    type("Password", "new-secret");
    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => expect(emailPassword.mock.calls).toEqual([[true], [false]]));
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
    expect(sendTestEmail).toHaveBeenCalledWith(draft, false);
    expect(readSettings().kindleEmail).toEqual(EMPTY_EMAIL);
    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => expect(readSettings().kindleEmail).toEqual(draft));
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

  it("shows why the test failed", async () => {
    const sendTestEmail = vi.fn(async () => {
      throw new Error("Couldn't reach the mail server at smtp.example.test:587");
    });
    setBackend({ sendTestEmail } as unknown as Backend);
    render(<EmailSettingsForm />);
    type("SMTP server", "smtp.example.test");
    fireEvent.click(screen.getByRole("button", { name: /Send test email/ }));
    expect(await screen.findByText(/Couldn't reach the mail server/)).toBeTruthy();
  });

  it("shows a locked keychain on Send test email, asking for the saved password", async () => {
    const locked =
      "The system keychain is locked or can't be read, so the SMTP password isn't available. Unlock it and try again (prompt dismissed)";
    const sendTestEmail = vi.fn(async (_s: unknown, _typed: boolean) => {
      throw new Error(locked);
    });
    setBackend({ sendTestEmail, emailPassword: keychain("locked") } as unknown as Backend);
    render(<EmailSettingsForm />);
    type("SMTP server", "smtp.example.test");
    type("User name", "me@example.test");
    fireEvent.click(screen.getByRole("button", { name: /Send test email/ }));
    expect(await screen.findByText(locked)).toBeTruthy();
    expect(sendTestEmail.mock.calls[0]?.[1]).toBe(false);
  });
});
