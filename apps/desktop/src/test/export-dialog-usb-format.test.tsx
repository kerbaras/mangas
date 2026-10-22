import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ExportDialog } from "@/components/export-dialog";
import { TooltipProvider } from "@/components/ui/tooltip";
import { setBackend } from "@/lib/ipc";
import type { UsbVerdict } from "@/lib/ipc.send";
import { chapters, enabledButton, exportBackend, solo } from "./export-backend";

afterEach(() => setBackend(null));

const kindle = { name: "Kindle Scribe", root: "/media/me/Kindle", documents: "/d" };
const dialog = () => (
  <TooltipProvider>
    <ExportDialog open onOpenChange={() => {}} picks={[{ manga: solo, chapters }]} />
  </TooltipProvider>
);

function backend(verdict: Omit<UsbVerdict, "device">) {
  const sendUsb = vi.fn(async (file: string) => ({ path: file, already_there: false }));
  const usbVerdict = vi.fn(async (_f: string, device: string) => ({ ...verdict, device }));
  const { exportVolumes } = exportBackend({
    findKindles: async () => [kindle],
    sendUsb,
    usbVerdict,
  });
  return { sendUsb, exportVolumes };
}

/** The dialog's main button, whatever it reads once a delivery stopped ("Retry copy"). */
const mainButton = () => screen.getByRole("button", { name: /Retry copy|Export 2 books/ });

describe("export dialog: formats a Kindle may not open over USB", () => {
  it("writes the EPUBs, warns before copying, and copies both on Copy anyway", async () => {
    const reason = "Kindle Scribe may not open an EPUB copied over USB.";
    const { sendUsb, exportVolumes } = backend({ decision: "warn", format: "EPUB", reason });
    render(dialog());
    fireEvent.mouseDown(await screen.findByRole("tab", { name: "Kindle over USB" }));
    fireEvent.click(await enabledButton(/Export 2 books and copy/));
    expect(await screen.findByText("Kindle Scribe may not open EPUB")).toBeTruthy();
    expect(sendUsb).not.toHaveBeenCalled();
    expect((mainButton() as HTMLButtonElement).disabled).toBe(true);
    fireEvent.click(screen.getByRole("button", { name: /Copy anyway/ }));
    await waitFor(() => expect(sendUsb).toHaveBeenCalledTimes(2));
    expect(exportVolumes).toHaveBeenCalledOnce();
    expect(sendUsb).toHaveBeenCalledWith("/k/Solo v1.epub", "Solo v1", kindle.root, true);
  });

  it("refuses to copy CBZs, offering no Copy anyway", async () => {
    const reason = "Kindle Scribe doesn't open CBZ files.";
    const { sendUsb, exportVolumes } = backend({ decision: "refused", format: "CBZ", reason });
    render(dialog());
    fireEvent.click(await screen.findByLabelText("CBZ"));
    fireEvent.mouseDown(await screen.findByRole("tab", { name: "Kindle over USB" }));
    fireEvent.click(await enabledButton(/Export 2 books and copy/));
    expect(await screen.findByText("Kindle Scribe doesn't open CBZ")).toBeTruthy();
    expect(screen.queryByRole("button", { name: /Copy anyway/ })).toBeNull();
    expect(sendUsb).not.toHaveBeenCalled();
    expect((mainButton() as HTMLButtonElement).disabled).toBe(true);
    expect(exportVolumes).toHaveBeenCalledOnce();
  });
});
