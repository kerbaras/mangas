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

const type = (label: string, value: string) =>
  fireEvent.change(screen.getByLabelText(label), { target: { value } });

describe("email settings", () => {
  it("says the password is in the keychain when there is one", async () => {
    const emailPasswordStorage = vi.fn(async () => "keychain");
    setBackend({ emailPasswordStorage } as unknown as Backend);
    writeSettings({ kindleEmail: WITH_PASSWORD });
    render(<EmailSettingsForm />);
    expect(await screen.findByText("Kept in your system keychain.")).toBeTruthy();
    expect(screen.queryByText(/stored unencrypted/)).toBeNull();
    expect((screen.getByLabelText("Password") as HTMLInputElement).type).toBe("password");
  });

  it("warns that the password is stored unencrypted without a keychain", async () => {
    const emailPasswordStorage = vi.fn(async () => "file");
    setBackend({ emailPasswordStorage } as unknown as Backend);
    writeSettings({ kindleEmail: WITH_PASSWORD });
    render(<EmailSettingsForm />);
    expect(
      await screen.findByText("No system keychain: the password is stored unencrypted"),
    ).toBeTruthy();
    expect(screen.getByText("Kept in the app settings (no system keychain).")).toBeTruthy();
    expect(screen.queryByText("Kept in your system keychain.")).toBeNull();
  });

  it("only claims no password is saved when the keychain answered", async () => {
    const emailPasswordStorage = vi.fn(async () => "file");
    setBackend({ emailPasswordStorage } as unknown as Backend);
    render(<EmailSettingsForm />);
    expect(await screen.findByText("No password in the app settings.")).toBeTruthy();
    expect(screen.queryByText("No password saved.")).toBeNull();
  });

  it("says no password is saved once it is cleared", async () => {
    const emailPasswordStorage = vi.fn(async () => "keychain");
    setBackend({ emailPasswordStorage } as unknown as Backend);
    writeSettings({ kindleEmail: WITH_PASSWORD });
    render(<EmailSettingsForm />);
    expect(await screen.findByText("Kept in your system keychain.")).toBeTruthy();
    act(() => writeSettings({ kindleEmail: EMPTY_EMAIL }));
    expect(await screen.findByText("No password saved.")).toBeTruthy();
    expect(screen.queryByText("Kept in your system keychain.")).toBeNull();
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
    expect(sendTestEmail).toHaveBeenCalledWith(draft);
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
});
