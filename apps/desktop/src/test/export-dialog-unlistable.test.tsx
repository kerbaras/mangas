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

  it("refuses a replacing export up front with the same line", async () => {
    const checkExportDir = unlistable();
    exportBackend({ checkExportDir });
    render(dialog());
    fireEvent.click(await screen.findByRole("switch", { name: "Replace existing files" }));
    expect(await screen.findByText(refused)).toBeTruthy();
    await waitFor(async () => expect((await exportButton()).hasAttribute("disabled")).toBe(true));
    expect(checkExportDir).toHaveBeenLastCalledWith(expect.stringMatching(/kindle$/), false, true);
  });

  it("re-enables Export once the folder can be listed again, with no option changed", async () => {
    let listable = false;
    const checkExportDir = vi.fn(async () => {
      if (!listable) throw refused;
    });
    exportBackend({ checkExportDir });
    render(dialog());
    fireEvent.click(await screen.findByRole("switch", { name: "Replace existing files" }));
    expect(await screen.findByText(refused)).toBeTruthy();
    listable = true;
    await waitFor(() => expect(screen.queryByText(refused)).toBeNull(), { timeout: 4000 });
    expect(await enabledButton(/Export 2 books/)).toBeTruthy();
  });

  it("reads the names again once a split export's folder lists", async () => {
    let listable = false;
    const checkExportDir = vi.fn(async () => {
      if (!listable) throw refused;
    });
    const freeExportNames = vi.fn(async (_dir: string, names: string[]) =>
      listable ? names.map((n) => n.replace(/\.(\w+)$/, " (2).$1")) : names,
    );
    exportBackend({ checkExportDir, freeExportNames });
    render(dialog());
    fireEvent.click(await screen.findByLabelText("Split into parts of"));
    expect(await screen.findByText(refused)).toBeTruthy();
    listable = true;
    expect(await screen.findByText("Solo v1 (2).epub", {}, { timeout: 4000 })).toBeTruthy();
    expect(screen.queryByText(refused)).toBeNull();
    expect(await enabledButton(/Export 2 books/)).toBeTruthy();
  });

  it("checks the folder again when the part size changes", async () => {
    let listable = true;
    const checkExportDir = vi.fn(async () => {
      if (!listable) throw refused;
    });
    exportBackend({ checkExportDir });
    render(dialog());
    fireEvent.click(await screen.findByLabelText("Split into parts of"));
    expect(await enabledButton(/Export 2 books/)).toBeTruthy();
    listable = false;
    fireEvent.change(screen.getByLabelText("Part size in MB"), { target: { value: "20" } });
    expect(await screen.findByText(refused)).toBeTruthy();
    await waitFor(async () => expect((await exportButton()).hasAttribute("disabled")).toBe(true));
  });
});
