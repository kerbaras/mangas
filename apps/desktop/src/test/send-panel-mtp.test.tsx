import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { SendPanel } from "@/components/send-panel";
import { type Backend, setBackend } from "@/lib/ipc";
import { MTP_HINT, type UsbVerdict } from "@/lib/ipc.send";
import { readSettings, writeSettings } from "@/lib/settings";

const FILE = "/books/Solo Leveling 1-3.pdf";
const DEVICE = "Kindle Paperwhite";

afterEach(() => {
  setBackend(null);
  writeSettings({ kindleEmail: { ...readSettings().kindleEmail, to: "", host: "" } });
});

const emailSetUp = () =>
  writeSettings({
    kindleEmail: {
      ...readSettings().kindleEmail,
      to: "reader@kindle.com",
      host: "smtp.test",
      username: "me@example.test",
    },
  });

/** Stands in for core's `usb_verdict`: the decision by extension, as `usb::SIDELOAD` has it. */
const usbVerdict = vi.fn(async (file: string, device: string): Promise<UsbVerdict> => {
  const format = (file.split(".").pop() ?? "").toUpperCase();
  const table: Record<string, UsbVerdict["decision"]> = { CBZ: "refused", EPUB: "warn" };
  const decision = table[format] ?? "supported";
  const reason = decision === "supported" ? null : `${device} won't take ${format}.`;
  return { decision, format, device, reason };
});

function stub(overrides: Partial<Backend>) {
  setBackend({
    onCopyProgress: async () => () => {},
    findKindles: vi.fn().mockResolvedValue([]),
    usbKindles: vi.fn().mockResolvedValue([{ name: DEVICE }]),
    usbVerdict,
    ...overrides,
  } as unknown as Backend);
}

describe("send panel, Kindle connected over MTP", () => {
  it("says MTP instead of Device not found and sends by email from there", async () => {
    emailSetUp();
    const sendUsb = vi.fn();
    const sendEmail = vi.fn().mockResolvedValue({ to: "reader@kindle.com", bytes: 1_000_000 });
    stub({ sendUsb, sendEmail });
    render(<SendPanel file={FILE} />);
    fireEvent.click(screen.getByRole("button", { name: /Copy over USB/ }));
    expect(await screen.findByText(`${DEVICE} connected over MTP`)).toBeTruthy();
    expect(screen.getByText(`${DEVICE} ${MTP_HINT}. Send it by email instead.`)).toBeTruthy();
    expect(usbVerdict).toHaveBeenCalledWith(FILE, DEVICE);
    expect(screen.queryByText("Device not found")).toBeNull();
    expect(sendUsb).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: /Use Email/ }));
    await waitFor(() => expect(sendEmail).toHaveBeenCalledWith(FILE, "Solo Leveling 1-3"));
    expect(await screen.findByText(/Emailed Solo Leveling 1-3.pdf/)).toBeTruthy();
  });

  it("offers no Use Email until email is set up, and keeps Device not found for no Kindle", async () => {
    const usbKindles = vi
      .fn()
      .mockResolvedValueOnce([{ name: DEVICE }])
      .mockResolvedValue([]);
    stub({ usbKindles });
    render(<SendPanel file={FILE} />);
    fireEvent.click(screen.getByRole("button", { name: /Copy over USB/ }));
    expect(await screen.findByText(`${DEVICE} connected over MTP`)).toBeTruthy();
    expect(screen.queryByRole("button", { name: /Use Email/ })).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: /Retry/ }));
    expect(await screen.findByText("Device not found")).toBeTruthy();
  });

  it("refuses a CBZ by its format, with no Email to fall back on", async () => {
    emailSetUp();
    stub({ sendUsb: vi.fn() });
    render(<SendPanel file="/b/Vol 1.cbz" />);
    fireEvent.click(screen.getByRole("button", { name: /Copy over USB/ }));
    expect(await screen.findByText(`${DEVICE} doesn't open CBZ`)).toBeTruthy();
    expect(screen.queryByText(/connected over MTP/)).toBeNull();
    expect(screen.queryByRole("button", { name: /Use Email|Copy anyway/ })).toBeNull();
  });

  it("warns about an EPUB first; Copy anyway then meets MTP, which offers Email", async () => {
    emailSetUp();
    stub({ sendUsb: vi.fn() });
    render(<SendPanel file="/b/Vol 1.epub" />);
    fireEvent.click(screen.getByRole("button", { name: /Copy over USB/ }));
    expect(await screen.findByText(`${DEVICE} may not open EPUB`)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: /Copy anyway/ }));
    expect(await screen.findByText(`${DEVICE} connected over MTP`)).toBeTruthy();
    expect(screen.getByRole("button", { name: /Use Email/ })).toBeTruthy();
  });

  it("names PDF or EPUB, not Email, for an AZW3 it can't copy over MTP", async () => {
    emailSetUp();
    stub({ sendUsb: vi.fn() });
    render(<SendPanel file="/b/Vol 1.azw3" />);
    fireEvent.click(screen.getByRole("button", { name: /Copy over USB/ }));
    expect(await screen.findByText(`${DEVICE} connected over MTP`)).toBeTruthy();
    const hint = `${DEVICE} ${MTP_HINT}. To send it by email, export the book as PDF or EPUB.`;
    expect(screen.getByText(hint)).toBeTruthy();
    expect(screen.queryByRole("button", { name: /Use Email/ })).toBeNull();
  });
});
