import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ExportDialog } from "@/components/export-dialog";
import { TooltipProvider } from "@/components/ui/tooltip";
import { setBackend } from "@/lib/ipc";
import { writeSettings } from "@/lib/settings";
import { chapters, exportBackend, solo } from "./export-backend";

afterEach(() => {
  setBackend(null);
  writeSettings({ defaultFormat: null, defaultDevice: "kindle-paperwhite12" });
});

const dialog = (onSent?: () => void) => (
  <TooltipProvider>
    <ExportDialog
      open
      onOpenChange={() => {}}
      picks={[{ manga: solo, chapters }]}
      onSent={onSent}
    />
  </TooltipProvider>
);

const exportButton = () => screen.findByRole("button", { name: /Export 2 books/ });

describe("export dialog: what it shows before and after writing", () => {
  it("previews the names a folder that already holds the books leaves free", async () => {
    let taken = 1;
    const freeExportNames = vi.fn(async (_dir: string, names: string[]) =>
      names.map((n) => (taken > 1 ? n.replace(/\.(\w+)$/, ` (${taken}).$1`) : n)),
    );
    const { exportVolumes } = exportBackend({ freeExportNames });
    render(dialog());
    expect(await screen.findByText("Solo v1.epub")).toBeTruthy();
    expect(freeExportNames).toHaveBeenCalledWith(expect.stringMatching(/kindle$/), [
      "Solo v1.epub",
      "Solo v2.epub",
    ]);
    taken = 2;
    fireEvent.click(await exportButton());
    await waitFor(() => expect(exportVolumes).toHaveBeenCalledOnce());
    expect(await screen.findByText("Solo v1 (2).epub")).toBeTruthy();
    fireEvent.click(await screen.findByLabelText("PDF"));
    expect(await screen.findByText("Solo v2 (2).pdf")).toBeTruthy();
  });

  it("keeps the books that copied next to the one that failed, and refreshes the history", async () => {
    const kindle = {
      name: "Kindle",
      root: "/media/me/Kindle",
      documents: "/media/me/Kindle/documents",
    };
    const sendUsb = vi
      .fn()
      .mockResolvedValueOnce({
        path: "/media/me/Kindle/documents/Solo v1.epub",
        already_there: false,
      })
      .mockRejectedValueOnce(new Error("No space left on device"));
    const onSent = vi.fn();
    exportBackend({ findKindles: async () => [kindle], sendUsb });
    render(dialog(onSent));
    fireEvent.mouseDown(await screen.findByRole("tab", { name: "Kindle over USB" }));
    fireEvent.click(await screen.findByRole("button", { name: /Export 2 books and copy/ }));
    expect(await screen.findByText("Copy failed")).toBeTruthy();
    expect(screen.getByText(/No space left on device/)).toBeTruthy();
    expect(screen.getByText(/Copied to Kindle: .*Solo v1\.epub/)).toBeTruthy();
    expect(onSent).toHaveBeenCalledOnce();
  });

  it("clears the last result once an option changes", async () => {
    const { exportVolumes } = exportBackend();
    render(dialog());
    fireEvent.click(await exportButton());
    await waitFor(() => expect(exportVolumes).toHaveBeenCalledOnce());
    expect(await screen.findByText("Wrote 2 books")).toBeTruthy();
    fireEvent.click(screen.getByLabelText("PDF"));
    await waitFor(() => expect(screen.queryByText("Wrote 2 books")).toBeNull());
  });
});
