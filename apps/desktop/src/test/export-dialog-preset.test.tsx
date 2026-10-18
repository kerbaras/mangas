import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ExportDialog } from "@/components/export-dialog";
import { TooltipProvider } from "@/components/ui/tooltip";
import { setBackend } from "@/lib/ipc";
import type { ExportPreset } from "@/lib/ipc.cut-preview";
import { writeSettings } from "@/lib/settings";
import { ch, chapters, enabledButton, exportBackend, solo } from "./export-backend";

afterEach(() => {
  setBackend(null);
  writeSettings({ defaultFormat: null, defaultDevice: "kindle-paperwhite12" });
});

const saved: ExportPreset = { profile: "kobo-clara", format: "cbz", cut_preset: "fewest-cuts" };

const withPreset = (preset: ExportPreset | null) => {
  const getExportPreset = vi.fn(async () => preset);
  return { getExportPreset, ...exportBackend({ getExportPreset }) };
};

const dialog = (picks = [{ manga: solo, chapters }]) => (
  <TooltipProvider>
    <ExportDialog open onOpenChange={() => {}} picks={picks} />
  </TooltipProvider>
);

describe("export dialog: series export preset", () => {
  it("says it uses the series preset and fills device, format and cut preset from it", async () => {
    const { exportVolumes, getExportPreset } = withPreset(saved);
    render(dialog());
    expect(await screen.findByText("Using series preset")).toBeTruthy();
    expect(getExportPreset).toHaveBeenCalledWith("mangadex", "m");
    expect(await screen.findByText("Solo v1.cbz")).toBeTruthy();
    expect(screen.getByLabelText("CBZ")).toHaveProperty("checked", true);
    fireEvent.click(await enabledButton(/Export 2 books/));
    await waitFor(() => expect(exportVolumes).toHaveBeenCalledOnce());
    expect(exportVolumes.mock.calls[0][0]).toMatchObject({
      device: "kobo-clara",
      format: "cbz",
      preset: "fewest-cuts",
    });
  });

  it("applies the preset to one file per chapter too", async () => {
    const { exportVolumes } = withPreset(saved);
    render(dialog());
    await screen.findByText("Using series preset");
    fireEvent.mouseDown(await screen.findByRole("tab", { name: "Per chapter" }));
    fireEvent.click(await enabledButton(/Export 3 books/));
    await waitFor(() => expect(exportVolumes).toHaveBeenCalledOnce());
    expect(exportVolumes.mock.calls[0][0]).toMatchObject({
      grouping: { kind: "per_chapter" },
      device: "kobo-clara",
      format: "cbz",
    });
  });

  it("lets a field picked in the dialog win over the preset", async () => {
    const { exportVolumes } = withPreset(saved);
    render(dialog());
    await screen.findByText("Solo v1.cbz");
    fireEvent.click(screen.getByLabelText("PDF"));
    expect(await screen.findByText("Solo v1.pdf")).toBeTruthy();
    fireEvent.click(await enabledButton(/Export 2 books/));
    await waitFor(() => expect(exportVolumes).toHaveBeenCalledOnce());
    expect(exportVolumes.mock.calls[0][0]).toMatchObject({ device: "kobo-clara", format: "pdf" });
  });

  it("shows no hint and the stored defaults when the series has no preset", async () => {
    const { exportVolumes } = withPreset(null);
    render(dialog());
    fireEvent.click(await enabledButton(/Export 2 books/));
    await waitFor(() => expect(exportVolumes).toHaveBeenCalledOnce());
    expect(screen.queryByText("Using series preset")).toBeNull();
    expect(exportVolumes.mock.calls[0][0]).toMatchObject({
      device: "kindle-paperwhite12",
      format: "epub",
    });
  });

  it("looks up no preset for chapters of several series", async () => {
    const { getExportPreset } = withPreset(saved);
    const other = { ...solo, id: "o", title: "Other" };
    render(
      dialog([
        { manga: solo, chapters: [ch("1", "1")] },
        { manga: other, chapters: [ch("9", null)] },
      ]),
    );
    expect(await screen.findByText("Solo +1 more.epub")).toBeTruthy();
    expect(screen.queryByText("Using series preset")).toBeNull();
    expect(getExportPreset).not.toHaveBeenCalled();
    expect(screen.getByRole("button", { name: /Smart-cut options/ })).toHaveProperty(
      "disabled",
      true,
    );
  });
});
