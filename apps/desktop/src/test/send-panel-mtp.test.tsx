import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { SendPanel } from "@/components/send-panel";
import { type Backend, setBackend } from "@/lib/ipc";
import { MTP_HINT } from "@/lib/ipc.send";
import { readSettings, writeSettings } from "@/lib/settings";

const FILE = "/books/Solo Leveling 1-3.epub";

afterEach(() => {
  setBackend(null);
  writeSettings({ kindleEmail: { ...readSettings().kindleEmail, to: "", host: "" } });
});

function stub(overrides: Partial<Backend>) {
  setBackend({ onCopyProgress: async () => () => {}, ...overrides } as unknown as Backend);
}

describe("send panel, Kindle connected over MTP", () => {
  it("says MTP instead of Device not found and sends by email from there", async () => {
    writeSettings({
      kindleEmail: {
        ...readSettings().kindleEmail,
        to: "reader@kindle.com",
        host: "smtp.test",
        username: "me@example.test",
      },
    });
    const sendUsb = vi.fn();
    const sendEmail = vi.fn().mockResolvedValue({ to: "reader@kindle.com", bytes: 1_000_000 });
    stub({
      findKindles: vi.fn().mockResolvedValue([]),
      usbKindles: vi.fn().mockResolvedValue([{ name: "Kindle" }]),
      sendUsb,
      sendEmail,
    });
    render(<SendPanel file={FILE} />);
    fireEvent.click(screen.getByRole("button", { name: /Copy over USB/ }));
    expect(await screen.findByText("Kindle connected over MTP")).toBeTruthy();
    expect(screen.getByText(`${MTP_HINT}.`)).toBeTruthy();
    expect(screen.queryByText("Device not found")).toBeNull();
    expect(sendUsb).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: /Use Email/ }));
    await waitFor(() => expect(sendEmail).toHaveBeenCalledWith(FILE, "Solo Leveling 1-3"));
    expect(await screen.findByText(/Emailed Solo Leveling 1-3.epub/)).toBeTruthy();
  });

  it("offers no Use Email until email is set up, and keeps Device not found for no Kindle", async () => {
    const usbKindles = vi
      .fn()
      .mockResolvedValueOnce([{ name: "Kindle" }])
      .mockResolvedValue([]);
    stub({ findKindles: vi.fn().mockResolvedValue([]), usbKindles });
    render(<SendPanel file={FILE} />);
    fireEvent.click(screen.getByRole("button", { name: /Copy over USB/ }));
    expect(await screen.findByText("Kindle connected over MTP")).toBeTruthy();
    expect(screen.queryByRole("button", { name: /Use Email/ })).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: /Retry/ }));
    expect(await screen.findByText("Device not found")).toBeTruthy();
  });
});
