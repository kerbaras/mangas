import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ExportDialog } from "@/components/export-dialog";
import { TooltipProvider } from "@/components/ui/tooltip";
import { setBackend } from "@/lib/ipc";
import type { EstimateRequest } from "@/lib/ipc.volumes";
import { readSettings, writeSettings } from "@/lib/settings";
import { chapters, enabledButton, exportBackend, solo } from "./export-backend";

// Radix Select scrolls the picked option into view, which jsdom lacks.
Element.prototype.scrollIntoView = vi.fn();

afterEach(() => {
  setBackend(null);
  writeSettings({ defaultFormat: null, defaultDevice: "kindle-paperwhite12", jpegQuality: {} });
});

const dialog = () => (
  <TooltipProvider>
    <ExportDialog open onOpenChange={() => {}} picks={[{ manga: solo, chapters }]} />
  </TooltipProvider>
);

// 32 MB a chapter at the device's quality, 16 MB at 50.
const estimateExport = vi.fn(async (r: EstimateRequest) => ({
  pages: r.chapterDirs.length * 60,
  bytes: r.chapterDirs.length * (r.quality === 50 ? 16_000_000 : 32_000_000),
}));

describe("export dialog: Send-to-Kindle size", () => {
  it("warns over 50 MB, and splitting or a lower quality reach the export", async () => {
    const { exportVolumes } = exportBackend({ estimateExport });
    render(dialog());
    fireEvent.mouseDown(await screen.findByRole("tab", { name: "One book" }));
    expect(await screen.findByText("About 96 MB, 180 pages")).toBeTruthy();
    expect(screen.getByRole("alert").textContent).toMatch(/refuses files over 50 MB/);
    expect(estimateExport.mock.lastCall?.[0].chapterDirs).toEqual(["/d/1", "/d/2", "/d/3"]);

    fireEvent.click(screen.getByLabelText("Split into parts of"));
    expect(await screen.findByText("About 96 MB, 180 pages, about 3 parts")).toBeTruthy();
    expect(screen.queryByRole("alert")).toBeNull();

    exportVolumes.mockResolvedValueOnce([
      {
        output: "/k/Solo (part 1).epub",
        parts: ["/k/Solo (part 1).epub", "/k/Solo (part 2).epub"],
        pages: 180,
        skipped: 0,
      },
    ]);
    fireEvent.click(await enabledButton(/Export book/));
    await waitFor(() => expect(exportVolumes).toHaveBeenCalledOnce());
    expect(exportVolumes.mock.calls[0][0]).toMatchObject({ splitMb: 45, quality: null });
  });

  it("re-estimates at the picked JPEG quality, keeps it for the device and exports with it", async () => {
    const { exportVolumes } = exportBackend({ estimateExport });
    render(dialog());
    fireEvent.mouseDown(await screen.findByRole("tab", { name: "One book" }));
    expect(await screen.findByText("About 96 MB, 180 pages")).toBeTruthy();
    fireEvent.keyDown(screen.getByRole("combobox", { name: "JPEG quality" }), { key: "Enter" });
    fireEvent.keyDown(await screen.findByRole("option", { name: "JPEG quality: 50" }), {
      key: "Enter",
    });
    expect(await screen.findByText("About 48 MB, 180 pages")).toBeTruthy();
    expect(screen.queryByRole("alert")).toBeNull();
    expect(readSettings().jpegQuality).toEqual({ "kindle-paperwhite12": 50 });

    fireEvent.click(await enabledButton(/Export book/));
    await waitFor(() => expect(exportVolumes).toHaveBeenCalledOnce());
    expect(exportVolumes.mock.calls[0][0]).toMatchObject({ quality: 50, splitMb: null });
  });

  it("starts at the quality stored for the device", async () => {
    await writeSettings({ jpegQuality: { "kindle-paperwhite12": 50 } });
    exportBackend({ estimateExport });
    render(dialog());
    fireEvent.mouseDown(await screen.findByRole("tab", { name: "One book" }));
    expect(await screen.findByText("About 48 MB, 180 pages")).toBeTruthy();
    expect(screen.getByRole("combobox", { name: "JPEG quality" }).textContent).toBe(
      "JPEG quality: 50",
    );
  });
});
