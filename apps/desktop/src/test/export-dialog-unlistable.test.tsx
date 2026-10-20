import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ExportDialog } from "@/components/export-dialog";
import { TooltipProvider } from "@/components/ui/tooltip";
import { setBackend } from "@/lib/ipc";
import { writeSettings } from "@/lib/settings";
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

const refused =
  "couldn't list /home/me/.local/share/mangas/kindle: Permission denied (os error 13)";

/** A folder that can be written to but not listed. */
const unlistable = () =>
  vi.fn(async (_dir: string, split: boolean, replace: boolean) => {
    if (split || replace) throw refused;
  });

const exportButton = () => screen.findByRole("button", { name: /Export 2 books/ });

describe("export dialog: a folder that can't be listed", () => {
  it("keeps a plain export and its planned names", async () => {
    const checkExportDir = unlistable();
    exportBackend({ checkExportDir });
    render(dialog());
    expect(await screen.findByText("Solo v1.epub")).toBeTruthy();
    expect(await enabledButton(/Export 2 books/)).toBeTruthy();
    expect(screen.queryByText(refused)).toBeNull();
    expect(checkExportDir).not.toHaveBeenCalled();
  });

  it("refuses a split export up front with the couldn't list line", async () => {
    exportBackend({ checkExportDir: unlistable() });
    render(dialog());
    fireEvent.click(await screen.findByLabelText("Split into parts of"));
    expect(await screen.findByText(refused)).toBeTruthy();
    await waitFor(async () => expect((await exportButton()).hasAttribute("disabled")).toBe(true));
  });

  it("refuses a replacing export up front, and lets it go once the folder lists", async () => {
    const checkExportDir = unlistable();
    exportBackend({ checkExportDir });
    render(dialog());
    fireEvent.click(await screen.findByRole("switch", { name: "Replace existing files" }));
    expect(await screen.findByText(refused)).toBeTruthy();
    await waitFor(async () => expect((await exportButton()).hasAttribute("disabled")).toBe(true));
    expect(checkExportDir).toHaveBeenLastCalledWith(expect.stringMatching(/kindle$/), false, true);
  });
});
