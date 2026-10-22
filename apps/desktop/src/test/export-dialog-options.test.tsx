import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ExportDialog } from "@/components/export-dialog";
import { TooltipProvider } from "@/components/ui/tooltip";
import { setBackend } from "@/lib/ipc";
import { writeSettings } from "@/lib/settings";
import { chapters, enabledButton, exportBackend, solo } from "./export-backend";

afterEach(() => {
  setBackend(null);
  writeSettings({ defaultFormat: null, defaultDevice: "kindle-paperwhite12" });
});

const dialog = () => (
  <TooltipProvider>
    <ExportDialog open onOpenChange={() => {}} picks={[{ manga: solo, chapters }]} />
  </TooltipProvider>
);

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

  it("offers email only for formats Send to Kindle takes", async () => {
    exportBackend();
    render(dialog());
    fireEvent.click(await screen.findByLabelText("CBZ"));
    const email = await screen.findByRole("tab", { name: "Email to Kindle" });
    expect(email.getAttribute("aria-disabled")).toBe("true");
    fireEvent.focus(email);
    const tip = await screen.findByRole("tooltip");
    expect(tip.textContent).toMatch(/takes EPUB or PDF, not a CBZ/);
    fireEvent.mouseDown(email);
    expect(email.getAttribute("aria-selected")).toBe("false");
    expect(screen.getByRole("tab", { name: "Save to folder" }).getAttribute("aria-selected")).toBe(
      "true",
    );
  });
});
