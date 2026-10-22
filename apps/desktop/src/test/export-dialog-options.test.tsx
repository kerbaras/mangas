import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ExportDialog } from "@/components/export-dialog";
import { TooltipProvider } from "@/components/ui/tooltip";
import { setBackend } from "@/lib/ipc";
import { readSettings, writeSettings } from "@/lib/settings";
import { chapters, enabledButton, exportBackend, solo } from "./export-backend";

afterEach(() => {
  setBackend(null);
  writeSettings({
    defaultFormat: null,
    defaultDevice: "kindle-paperwhite12",
    kindleEmail: { ...readSettings().kindleEmail, to: "", host: "" },
  });
});

const dialog = () => (
  <TooltipProvider>
    <ExportDialog open onOpenChange={() => {}} picks={[{ manga: solo, chapters }]} />
  </TooltipProvider>
);

const checked = (label: string) => (screen.getByLabelText(label) as HTMLInputElement).checked;
const exportButton = () => enabledButton(/Export 2 books/);

describe("export dialog: format, smart cut and delivery", () => {
  it("writes the picked format and names the files with its extension", async () => {
    const { exportVolumes } = exportBackend();
    render(dialog());
    fireEvent.click(await screen.findByLabelText("CBZ"));
    expect(await screen.findByText("Solo v1.cbz")).toBeTruthy();
    fireEvent.click(await exportButton());
    await waitFor(() => expect(exportVolumes).toHaveBeenCalledOnce());
    expect(exportVolumes.mock.calls[0][0].format).toBe("cbz");
  });

  it("copies every book to the Kindle over USB as AZW3 and says when one is already there", async () => {
    const kindle = {
      name: "Kindle",
      root: "/media/me/Kindle",
      documents: "/media/me/Kindle/documents",
      ejectable: true,
    };
    const sendUsb = vi
      .fn()
      .mockResolvedValueOnce({
        path: "/media/me/Kindle/documents/Solo v1.azw3",
        already_there: true,
      })
      .mockResolvedValueOnce({
        path: "/media/me/Kindle/documents/Solo v2.azw3",
        already_there: false,
      });
    exportBackend({ findKindles: async () => [kindle], sendUsb });
    render(dialog());
    fireEvent.mouseDown(await screen.findByRole("tab", { name: "Kindle over USB" }));
    fireEvent.click(await enabledButton(/Export 2 books and copy/));
    await waitFor(() => expect(sendUsb).toHaveBeenCalledTimes(2));
    expect(sendUsb).toHaveBeenCalledWith("/k/Solo v1.azw3", "Solo v1", kindle.root);
    expect(await screen.findByText(/Already on Kindle: .*Solo v1\.azw3/)).toBeTruthy();
    expect(screen.getByText(/Copied to Kindle: .*Solo v2\.azw3/)).toBeTruthy();
  });

  it("emails an EPUB when the picked format is one Send to Kindle doesn't take", async () => {
    const { exportVolumes } = exportBackend();
    writeSettings({
      kindleEmail: { ...readSettings().kindleEmail, to: "r@kindle.com", host: "h", from: "m@x.t" },
    });
    render(dialog());
    fireEvent.click(await screen.findByLabelText("CBZ"));
    const email = await screen.findByRole("tab", { name: "Email to Kindle" });
    expect(email.getAttribute("aria-disabled")).toBeNull();
    fireEvent.mouseDown(email);
    expect(email.getAttribute("aria-selected")).toBe("true");
    const why = "Email sends EPUB; Send to Kindle doesn't take CBZ by email.";
    expect(await screen.findByText(why)).toBeTruthy();
    await waitFor(() => expect(checked("EPUB")).toBe(true));
    expect(await screen.findByText("Solo v1.epub")).toBeTruthy();
    fireEvent.mouseDown(screen.getByRole("tab", { name: "Save to folder" }));
    await waitFor(() => expect(checked("CBZ")).toBe(true));
    fireEvent.click(await exportButton());
    await waitFor(() => expect(exportVolumes).toHaveBeenCalled());
    expect(exportVolumes.mock.calls[0][0].format).toBe("cbz");
  });
});
