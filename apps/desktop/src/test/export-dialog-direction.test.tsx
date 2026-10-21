import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ExportDialog } from "@/components/export-dialog";
import { TooltipProvider } from "@/components/ui/tooltip";
import { type Manga, setBackend } from "@/lib/ipc";
import type { ExportPreset } from "@/lib/ipc.cut-preview";
import { chapters, enabledButton, exportBackend, solo } from "./export-backend";

// Radix Select scrolls the picked option into view, which jsdom lacks.
Element.prototype.scrollIntoView = vi.fn();

afterEach(() => setBackend(null));

const manga: Manga = { ...solo, direction: "rtl" };
const dialog = () => (
  <TooltipProvider>
    <ExportDialog open onOpenChange={() => {}} picks={[{ manga, chapters }]} />
  </TooltipProvider>
);

function backend(saved: ExportPreset | null) {
  let stored = saved;
  const setExportPreset = vi.fn(async (_s: string, _id: string, p: ExportPreset) => {
    stored = p;
  });
  const { exportVolumes } = exportBackend({
    getExportPreset: async () => stored,
    setExportPreset,
  });
  return { exportVolumes, setExportPreset };
}

const pageTurn = () => screen.findByRole("combobox", { name: "Page turn" });

describe("export dialog: page turn", () => {
  it("shows the source's direction and leaves it to the series", async () => {
    const { exportVolumes } = backend(null);
    render(dialog());
    await waitFor(async () => expect((await pageTurn()).textContent).toBe("Right to left (manga)"));
    expect(screen.getByText("From the source; picking one saves it for this series.")).toBeTruthy();
    fireEvent.click(await enabledButton(/Export/));
    await waitFor(() => expect(exportVolumes).toHaveBeenCalledOnce());
    expect(exportVolumes.mock.calls[0][0]).toMatchObject({ direction: null });
  });

  it("saves a picked direction for the series and exports with it", async () => {
    const { exportVolumes, setExportPreset } = backend({ page_height: 0.8 });
    render(dialog());
    fireEvent.keyDown(await pageTurn(), { key: "Enter" });
    fireEvent.keyDown(await screen.findByRole("option", { name: "Left to right" }), {
      key: "Enter",
    });
    await waitFor(() => expect(setExportPreset).toHaveBeenCalledOnce());
    expect(setExportPreset.mock.calls[0][2]).toEqual({ page_height: 0.8, direction: "ltr" });
    expect(await screen.findByText("Saved for this series.")).toBeTruthy();
    fireEvent.click(await enabledButton(/Export/));
    await waitFor(() => expect(exportVolumes).toHaveBeenCalledOnce());
    expect(exportVolumes.mock.calls[0][0]).toMatchObject({ direction: "ltr" });
  });
});
