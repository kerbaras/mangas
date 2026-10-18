import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { SendPanel } from "@/components/send-panel";
import { type Backend, setBackend } from "@/lib/ipc";
import type { CopyProgress, UsbCopy } from "@/lib/ipc.send";
import { readSettings, writeSettings } from "@/lib/settings";

const kindle = {
  name: "Kindle",
  root: "/media/me/Kindle",
  documents: "/media/me/Kindle/documents",
};
const FILE = "/books/Solo Leveling 1-3.epub";

afterEach(() => {
  setBackend(null);
  writeSettings({ kindleEmail: { ...readSettings().kindleEmail, to: "", host: "" } });
});

function stub(overrides: Partial<Backend>) {
  let progress: ((p: CopyProgress) => void) | null = null;
  setBackend({
    onCopyProgress: async (cb: (p: CopyProgress) => void) => {
      progress = cb;
      return () => {};
    },
    ...overrides,
  } as unknown as Backend);
  return () => progress;
}

describe("send panel", () => {
  it("shows Device not found, then copies once Retry finds the Kindle", async () => {
    const findKindles = vi.fn().mockResolvedValueOnce([]).mockResolvedValue([kindle]);
    let finish: (copy: UsbCopy) => void = () => {};
    const sendUsb = vi.fn(() => new Promise<UsbCopy>((r) => (finish = r)));
    const progress = stub({ findKindles, sendUsb });
    const onSent = vi.fn();
    render(<SendPanel file={FILE} onSent={onSent} />);
    fireEvent.click(screen.getByRole("button", { name: /Copy over USB/ }));
    expect(await screen.findByText("Device not found")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: /Retry/ }));
    await waitFor(() =>
      expect(sendUsb).toHaveBeenCalledWith(FILE, "Solo Leveling 1-3", kindle.root),
    );
    progress()?.({ done: 5_000_000, total: 10_000_000 });
    expect(await screen.findByText("5.0 MB / 10.0 MB")).toBeTruthy();
    finish({ path: "/media/me/Kindle/documents/Solo Leveling 1-3.epub", already_there: false });
    expect(await screen.findByText(/Copied to Kindle:/)).toBeTruthy();
    expect(onSent).toHaveBeenCalledTimes(1);
  });

  it("says the book is already on the Kindle when nothing was copied", async () => {
    const onboard = `${kindle.documents}/Solo Leveling 1-3.epub`;
    const findKindles = vi.fn().mockResolvedValue([kindle]);
    const sendUsb = vi.fn().mockResolvedValue({ path: onboard, already_there: true });
    stub({ findKindles, sendUsb });
    render(<SendPanel file={onboard} />);
    fireEvent.click(screen.getByRole("button", { name: /Copy over USB/ }));
    expect(await screen.findByText(`Already on Kindle: ${onboard}`)).toBeTruthy();
    expect(screen.queryByText(/Copied to/)).toBeNull();
  });

  it("asks to set up email until the address and server are filled in", () => {
    stub({});
    render(<SendPanel file={FILE} />);
    const button = screen.getByRole("button", { name: /Email to Kindle/ }) as HTMLButtonElement;
    expect(button.disabled).toBe(true);
    expect(screen.getByRole("button", { name: "Set up email" })).toBeTruthy();
  });

  it("shows the server's refusal when the email fails", async () => {
    writeSettings({
      kindleEmail: {
        ...readSettings().kindleEmail,
        to: "reader@kindle.com",
        host: "smtp.example.test",
        username: "me@example.test",
      },
    });
    const sendEmail = vi.fn(async () => {
      throw new Error("The mail server refused the message as too large");
    });
    stub({ sendEmail });
    render(<SendPanel file={FILE} />);
    fireEvent.click(screen.getByRole("button", { name: /Email to Kindle/ }));
    expect(await screen.findByText("Email not sent")).toBeTruthy();
    expect(screen.getByText(/too large/)).toBeTruthy();
    expect(sendEmail).toHaveBeenCalledWith(FILE, "Solo Leveling 1-3");
  });
});
