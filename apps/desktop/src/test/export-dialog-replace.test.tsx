import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ExportDialog } from "@/components/export-dialog";
import { TooltipProvider } from "@/components/ui/tooltip";
import { setBackend } from "@/lib/ipc";
import { readSettings, writeSettings } from "@/lib/settings";
import { chapters, enabledButton, exportBackend, solo } from "./export-backend";

afterEach(() => {
  setBackend(null);
  writeSettings({ replaceExisting: false, defaultDevice: "kindle-paperwhite12" });
});

beforeEach(() => writeSettings({ dataDir: "/home/me/.local/share/mangas" }));

const dialog = () => (
  <TooltipProvider>
    <ExportDialog open onOpenChange={() => {}} picks={[{ manga: solo, chapters }]} />
  </TooltipProvider>
);

/** The folder already holds both books, so the writer would number them. */
const taken = () =>
  vi.fn(async (_dir: string, names: string[]) =>
    names.map((n) => n.replace(/\.(\w+)$/, " (2).$1")),
  );

describe("export dialog: Replace existing files", () => {
  it("is off by default and exports numbered copies", async () => {
    const { exportVolumes } = exportBackend({ freeExportNames: taken() });
    render(dialog());
    const toggle = await screen.findByRole("switch", { name: "Replace existing files" });
    expect(toggle.getAttribute("aria-checked")).toBe("false");
    expect(await screen.findByText("Solo v1 (2).epub")).toBeTruthy();
    fireEvent.click(await enabledButton(/Export 2 books/));
    await waitFor(() => expect(exportVolumes).toHaveBeenCalledOnce());
    expect(exportVolumes.mock.calls[0]?.[0]).toMatchObject({ replace: false });
  });

  it("when on, previews the exact names, exports with replace and is remembered", async () => {
    const freeExportNames = taken();
    const { exportVolumes } = exportBackend({ freeExportNames });
    render(dialog());
    fireEvent.click(await screen.findByRole("switch", { name: "Replace existing files" }));
    expect(readSettings().replaceExisting).toBe(true);
    expect(await screen.findByText("Solo v1.epub")).toBeTruthy();
    expect(screen.queryByText("Solo v1 (2).epub")).toBeNull();
    fireEvent.click(await enabledButton(/Export 2 books/));
    await waitFor(() => expect(exportVolumes).toHaveBeenCalledOnce());
    expect(exportVolumes.mock.calls[0]?.[0]).toMatchObject({ replace: true });
  });

  it("starts on when the setting was saved on", async () => {
    writeSettings({ replaceExisting: true });
    exportBackend({ freeExportNames: taken() });
    render(dialog());
    const toggle = await screen.findByRole("switch", { name: "Replace existing files" });
    expect(toggle.getAttribute("aria-checked")).toBe("true");
  });
});
