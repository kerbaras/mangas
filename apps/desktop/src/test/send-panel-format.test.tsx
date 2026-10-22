import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { SendPanel } from "@/components/send-panel";
import { type Backend, setBackend } from "@/lib/ipc";
import type { UsbVerdict } from "@/lib/ipc.send";

const kindle = { name: "Kindle Paperwhite", root: "/media/me/Kindle", documents: "/d" };

afterEach(() => setBackend(null));

function stub(verdict: Omit<UsbVerdict, "device">) {
  const sendUsb = vi.fn(async (file: string) => ({ path: file, already_there: false }));
  const usbVerdict = vi.fn(async (_file: string, device: string) => ({ ...verdict, device }));
  setBackend({
    onCopyProgress: async () => () => {},
    findKindles: async () => [kindle],
    usbVerdict,
    sendUsb,
  } as unknown as Backend);
  return { sendUsb, usbVerdict };
}

describe("send panel, formats a Kindle may not open over USB", () => {
  it("warns about an EPUB, names the device, and copies only on Copy anyway", async () => {
    const reason =
      "Kindle Paperwhite may not open an EPUB copied over USB. Email it instead (Send to Kindle converts EPUB), or export the book as PDF or AZW3.";
    const { sendUsb, usbVerdict } = stub({ decision: "warn", format: "EPUB", reason });
    render(<SendPanel file="/b/Vol 1.epub" />);
    fireEvent.click(screen.getByRole("button", { name: /Copy over USB/ }));
    expect(await screen.findByText("Kindle Paperwhite may not open EPUB")).toBeTruthy();
    expect(screen.getByText(reason)).toBeTruthy();
    expect(usbVerdict).toHaveBeenCalledWith("/b/Vol 1.epub", "Kindle Paperwhite");
    expect(sendUsb).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: /Copy anyway/ }));
    await waitFor(() =>
      expect(sendUsb).toHaveBeenCalledWith("/b/Vol 1.epub", "Vol 1", kindle.root, true),
    );
    expect(await screen.findByText("Copied to Kindle Paperwhite: /b/Vol 1.epub")).toBeTruthy();
  });

  it("refuses a CBZ with no copy and no Copy anyway", async () => {
    const reason =
      "Kindle Paperwhite doesn't open CBZ files. To copy it over USB, export the book as PDF or AZW3.";
    const { sendUsb } = stub({ decision: "refused", format: "CBZ", reason });
    render(<SendPanel file="/b/Vol 1.cbz" />);
    fireEvent.click(screen.getByRole("button", { name: /Copy over USB/ }));
    expect(await screen.findByText("Kindle Paperwhite doesn't open CBZ")).toBeTruthy();
    expect(screen.getByText(reason)).toBeTruthy();
    expect(screen.queryByRole("button", { name: /Copy anyway/ })).toBeNull();
    expect(screen.queryByRole("button", { name: /Retry/ })).toBeNull();
    expect(sendUsb).not.toHaveBeenCalled();
  });

  it("copies a PDF straight away, as before", async () => {
    const { sendUsb } = stub({ decision: "supported", format: "PDF", reason: null });
    render(<SendPanel file="/b/Vol 1.pdf" />);
    fireEvent.click(screen.getByRole("button", { name: /Copy over USB/ }));
    await waitFor(() => expect(sendUsb).toHaveBeenCalledWith("/b/Vol 1.pdf", "Vol 1", kindle.root));
    expect(screen.queryByRole("alert")).toBeNull();
  });
});
