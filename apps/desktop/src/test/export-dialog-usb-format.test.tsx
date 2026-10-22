import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ExportDialog } from "@/components/export-dialog";
import { TooltipProvider } from "@/components/ui/tooltip";
import { setBackend } from "@/lib/ipc";
import type { UsbVerdict } from "@/lib/ipc.send";
import { readSettings, writeSettings } from "@/lib/settings";
import { chapters, enabledButton, exportBackend, solo } from "./export-backend";

afterEach(() => {
  setBackend(null);
  writeSettings({
    defaultFormat: null,
    kindleEmail: { ...readSettings().kindleEmail, to: "", host: "" },
  });
});

const emailSetUp = () =>
  writeSettings({
    kindleEmail: {
      ...readSettings().kindleEmail,
      to: "r@kindle.com",
      host: "smtp.test",
      from: "me@x.test",
    },
  });
const checked = (label: string) => (screen.getByLabelText(label) as HTMLInputElement).checked;

const kindle = { name: "Kindle Scribe", root: "/media/me/Kindle", documents: "/d" };
const dialog = () => (
  <TooltipProvider>
    <ExportDialog open onOpenChange={() => {}} picks={[{ manga: solo, chapters }]} />
  </TooltipProvider>
);

function backend(verdict: Omit<UsbVerdict, "device">, mtpOnly = false) {
  const sendUsb = vi.fn(async (file: string) => ({ path: file, already_there: false }));
  const usbVerdict = vi.fn(async (_f: string, device: string) => ({ ...verdict, device }));
  const { exportVolumes } = exportBackend({
    findKindles: async () => (mtpOnly ? [] : [kindle]),
    usbKindles: async () => [{ name: "Kindle Paperwhite" }],
    sendUsb,
    usbVerdict,
  });
  return { sendUsb, exportVolumes, usbVerdict };
}

describe("export dialog: the Kindle format follows the delivery", () => {
  it("writes and copies AZW3 over USB and picks EPUB for Email, saying why", async () => {
    emailSetUp();
    const { sendUsb, exportVolumes } = backend({
      decision: "supported",
      format: "AZW3",
      reason: null,
    });
    render(dialog());
    await waitFor(() => expect(checked("EPUB")).toBe(true));
    fireEvent.mouseDown(await screen.findByRole("tab", { name: "Email to Kindle" }));
    expect(await screen.findByText("Kindle over USB: AZW3; Email: EPUB.")).toBeTruthy();
    expect(checked("EPUB")).toBe(true);
    fireEvent.mouseDown(screen.getByRole("tab", { name: "Kindle over USB" }));
    await waitFor(() => expect(checked("AZW3 (Kindle)")).toBe(true));
    expect(screen.getByText("Kindle over USB: AZW3; Email: EPUB.")).toBeTruthy();
    fireEvent.click(await enabledButton(/Export 2 books and copy/));
    await waitFor(() => expect(sendUsb).toHaveBeenCalledTimes(2));
    expect(exportVolumes).toHaveBeenCalledWith(expect.objectContaining({ format: "azw3" }));
    expect(sendUsb).toHaveBeenCalledWith("/k/Solo v1.azw3", "Solo v1", kindle.root);
  });
});

describe("export dialog: formats a Kindle doesn't open over USB", () => {
  it.each([
    ["mounted", false, "Kindle Scribe"],
    ["MTP only", true, "Kindle Paperwhite"],
  ])(
    "refuses a picked EPUB before writing it (%s), naming AZW3 and Email",
    async (_, mtpOnly, device) => {
      const reason = `${device} doesn't open an EPUB copied over USB. Export the book as AZW3 to copy it, or send the EPUB by Email (Send to Kindle converts it).`;
      const verdict = { decision: "refused", format: "EPUB", reason } as const;
      const { sendUsb, exportVolumes, usbVerdict } = backend(verdict, mtpOnly);
      render(dialog());
      fireEvent.mouseDown(await screen.findByRole("tab", { name: "Kindle over USB" }));
      await waitFor(() => expect(checked("AZW3 (Kindle)")).toBe(true));
      fireEvent.click(screen.getByLabelText("EPUB"));
      expect(checked("EPUB")).toBe(true);
      fireEvent.click(await enabledButton(/Export 2 books and copy/));
      expect(await screen.findByText(`${device} doesn't open EPUB`)).toBeTruthy();
      expect(screen.getByText(reason)).toBeTruthy();
      expect(usbVerdict).toHaveBeenCalledWith("book.epub", device);
      expect(screen.queryByRole("button", { name: /Copy anyway/ })).toBeNull();
      expect(sendUsb).not.toHaveBeenCalled();
      expect(exportVolumes).not.toHaveBeenCalled();
    },
  );

  it.each([
    ["mounted", false, "Kindle Scribe"],
    ["MTP only", true, "Kindle Paperwhite"],
  ])(
    "refuses CBZs before writing them (%s), offering no Copy anyway or Email",
    async (_, mtpOnly, device) => {
      const reason = `${device} doesn't open CBZ files.`;
      const verdict = { decision: "refused", format: "CBZ", reason } as const;
      const { sendUsb, exportVolumes, usbVerdict } = backend(verdict, mtpOnly);
      render(dialog());
      fireEvent.mouseDown(await screen.findByRole("tab", { name: "Kindle over USB" }));
      await waitFor(() => expect(checked("AZW3 (Kindle)")).toBe(true));
      fireEvent.click(screen.getByLabelText("CBZ"));
      fireEvent.click(await enabledButton(/Export 2 books and copy/));
      expect(await screen.findByText(`${device} doesn't open CBZ`)).toBeTruthy();
      expect(usbVerdict).toHaveBeenCalledWith("book.cbz", device);
      expect(screen.queryByRole("button", { name: /Copy anyway|Use Email/ })).toBeNull();
      expect(screen.queryByText(/connected over MTP/)).toBeNull();
      expect(sendUsb).not.toHaveBeenCalled();
      expect(exportVolumes).not.toHaveBeenCalled();
    },
  );
});
