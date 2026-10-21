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

  it("goes back to the saved way when saving the pick fails", async () => {
    const setExportPreset = vi.fn(async () => {
      throw new Error("library is read-only");
    });
    exportBackend({ getExportPreset: async () => null, setExportPreset });
    render(dialog());
    await waitFor(async () => expect((await pageTurn()).textContent).toBe("Right to left (manga)"));
    fireEvent.keyDown(await pageTurn(), { key: "Enter" });
    fireEvent.keyDown(await screen.findByRole("option", { name: "Left to right" }), {
      key: "Enter",
    });
    await waitFor(() => expect(setExportPreset).toHaveBeenCalledOnce());
    await waitFor(async () => expect((await pageTurn()).textContent).toBe("Right to left (manga)"));
  });

  it("keeps a later pick when an earlier save fails after it", async () => {
    let failFirst: (e: Error) => void = () => {};
    const setExportPreset = vi
      .fn()
      .mockImplementationOnce(() => new Promise<void>((_, reject) => (failFirst = reject)))
      .mockResolvedValue(undefined);
    const { exportVolumes } = exportBackend({ getExportPreset: async () => null, setExportPreset });
    render(dialog());
    const pick = async (name: string) => {
      fireEvent.keyDown(await pageTurn(), { key: "Enter" });
      fireEvent.keyDown(await screen.findByRole("option", { name }), { key: "Enter" });
    };
    await pick("Left to right");
    await pick("Right to left (manga)");
    await waitFor(() => expect(setExportPreset).toHaveBeenCalledTimes(2));
    failFirst(new Error("library is read-only"));
    fireEvent.click(await enabledButton(/Export/));
    await waitFor(() => expect(exportVolumes).toHaveBeenCalledOnce());
    expect(exportVolumes.mock.calls[0][0]).toMatchObject({ direction: "rtl" });
  });

  it("starts another picked series from its own way", async () => {
    const presets = new Map<string, ExportPreset>();
    exportBackend({
      getExportPreset: async (_s: string, id: string) => presets.get(id) ?? null,
      setExportPreset: async (_s: string, id: string, p: ExportPreset) => {
        presets.set(id, p);
      },
    });
    const view = render(dialog());
    fireEvent.keyDown(await pageTurn(), { key: "Enter" });
    fireEvent.keyDown(await screen.findByRole("option", { name: "Left to right" }), {
      key: "Enter",
    });
    expect(await screen.findByText("Saved for this series.")).toBeTruthy();
    const other: Manga = { ...manga, id: "other", title: "Other" };
    view.rerender(
      <TooltipProvider>
        <ExportDialog open onOpenChange={() => {}} picks={[{ manga: other, chapters }]} />
      </TooltipProvider>,
    );
    await waitFor(async () => expect((await pageTurn()).textContent).toBe("Right to left (manga)"));
    expect(presets.has("other")).toBe(false);
  });
});
