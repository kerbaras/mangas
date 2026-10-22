import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { SendPanel } from "@/components/send-panel";
import { type Backend, setBackend } from "@/lib/ipc";
import { COPY_CANCELLED, type CopyProgress, type UsbCopy, type UsbVerdict } from "@/lib/ipc.send";
import { emailRefusal } from "./email-refusal-stub";

const FILE = "/books/Solo Leveling 1-3.azw3";
const DEVICE = "Kindle Paperwhite";

afterEach(() => setBackend(null));

/** Stands in for core's `usb_verdict`: the decision by extension, as `usb::SIDELOAD` has it. */
const usbVerdict = vi.fn(async (file: string, device: string): Promise<UsbVerdict> => {
  const format = (file.split(".").pop() ?? "").toUpperCase();
  const table: Record<string, UsbVerdict["decision"]> = { CBZ: "refused", EPUB: "refused" };
  const decision = table[format] ?? "supported";
  const reason = decision === "supported" ? null : `${device} doesn't open ${format} files.`;
  return { decision, format, device, reason };
});

function stub(overrides: Partial<Backend>) {
  const listeners: ((p: CopyProgress) => void)[] = [];
  setBackend({
    onCopyProgress: async (cb: (p: CopyProgress) => void) => {
      listeners.push(cb);
      return () => {};
    },
    findKindles: vi.fn().mockResolvedValue([]),
    usbKindles: vi.fn().mockResolvedValue([{ name: DEVICE, copyable: true }]),
    emailRefusal,
    usbVerdict,
    ...overrides,
  } as unknown as Backend);
  return (p: CopyProgress) => {
    for (const cb of listeners) cb(p);
  };
}

describe("send panel, Kindle over MTP on a build that copies to it", () => {
  it("copies with no mounted folder and says where the book is", async () => {
    const sendUsb = vi.fn().mockResolvedValue({ path: "documents/x.azw3", already_there: false });
    stub({ sendUsb });
    render(<SendPanel file={FILE} />);
    fireEvent.click(screen.getByRole("button", { name: /Copy over USB/ }));
    expect(await screen.findByText(`Copied to ${DEVICE}: documents/x.azw3`)).toBeTruthy();
    expect(sendUsb).toHaveBeenCalledWith(FILE, "Solo Leveling 1-3", undefined);
    expect(screen.queryByText(/connected over MTP/)).toBeNull();
  });

  it("refuses an EPUB by the format table before any upload", async () => {
    const sendUsb = vi.fn();
    stub({ sendUsb });
    render(<SendPanel file="/books/Solo Leveling 1-3.epub" />);
    fireEvent.click(screen.getByRole("button", { name: /Copy over USB/ }));
    expect(await screen.findByText(`${DEVICE} doesn't open EPUB files.`)).toBeTruthy();
    expect(sendUsb).not.toHaveBeenCalled();
  });

  it("cancels mid-copy: Cancel asks the backend and the copy reports it stopped", async () => {
    let reject: (e: Error) => void = () => {};
    const sendUsb = vi.fn((): Promise<UsbCopy> => new Promise((_, r) => (reject = r)));
    const cancelSendUsb = vi.fn(async () => reject(new Error(COPY_CANCELLED)));
    const progress = stub({ sendUsb, cancelSendUsb });
    render(<SendPanel file={FILE} />);
    fireEvent.click(screen.getByRole("button", { name: /Copy over USB/ }));
    await waitFor(() => expect(sendUsb).toHaveBeenCalled());
    progress({ done: 4_000_000, total: 18_400_000 });
    expect(await screen.findByText(`Copying to ${DEVICE}…`)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: /Cancel/ }));
    await waitFor(() => expect(cancelSendUsb).toHaveBeenCalledOnce());
    expect(await screen.findByText(COPY_CANCELLED)).toBeTruthy();
    expect(screen.getByText("Copy cancelled")).toBeTruthy();
    expect(screen.queryByText("Copy failed")).toBeNull();
    expect(screen.getByRole("alert").className).not.toContain("destructive");
    expect(screen.getByRole("button", { name: /Retry/ })).toBeTruthy();
  });
});
