import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ExportDialog } from "@/components/export-dialog";
import { ExportStoppedPanel } from "@/components/export-stopped-panel";
import { TooltipProvider } from "@/components/ui/tooltip";
import { ExportStopped, outcomeResults } from "@/lib/export-stopped";
import { setBackend } from "@/lib/ipc";
import type { ExportStats } from "@/lib/ipc.wire";
import { chapters, enabledButton, exportBackend, solo } from "./export-backend";

const kindle = {
  name: "Kindle",
  root: "/media/me/Kindle",
  documents: "/media/me/Kindle/documents",
};
const stats = (output: string): ExportStats => ({
  pages: 6,
  strips: 1,
  source_rows: 1,
  pagination_ms: 0,
  filters_ms: 0,
  skipped: 0,
  output,
});
const failed = { title: "Solo v2", output: "/k/Solo v2.epub", reason: "No space left on device" };
const stopped = () =>
  new ExportStopped({ written: [stats("/k/Solo v1.epub")], failed, not_written: [] });

afterEach(() => setBackend(null));

const dialog = (onExported = vi.fn()) => (
  <TooltipProvider>
    <ExportDialog
      open
      onOpenChange={() => {}}
      picks={[{ manga: solo, chapters }]}
      onExported={onExported}
    />
  </TooltipProvider>
);

describe("export dialog: a volume that fails part-way (#220)", () => {
  it("names the failed book, keeps the ones written before it and sends only those", async () => {
    const onExported = vi.fn();
    const sendUsb = vi.fn(async (_file: string) => ({
      path: "/media/me/Kindle/documents/Solo v1.epub",
      already_there: false,
    }));
    exportBackend({
      exportVolumes: vi.fn(async () => {
        throw stopped();
      }),
      findKindles: async () => [kindle],
      sendUsb,
    });
    render(dialog(onExported));
    fireEvent.mouseDown(await screen.findByRole("tab", { name: "Kindle over USB" }));
    fireEvent.click(await enabledButton(/Export 2 books and copy/));
    expect(await screen.findByText("Export stopped")).toBeTruthy();
    expect(screen.queryByText("Export failed")).toBeNull();
    expect(
      screen.getByText("Solo v2.epub failed and was not saved: No space left on device"),
    ).toBeTruthy();
    expect(screen.getByText("Kept the 1 book written before it: Solo v1.epub.")).toBeTruthy();
    await waitFor(() => expect(sendUsb).toHaveBeenCalledOnce());
    expect(sendUsb.mock.calls[0][0]).toBe("/k/Solo v1.epub");
    expect(onExported).toHaveBeenCalledWith([
      expect.objectContaining({ output: "/k/Solo v1.epub" }),
    ]);
  });

  it("lists the books it didn't start and says so when nothing was written", () => {
    const none = new ExportStopped({
      written: [],
      failed: { ...failed, title: "Solo v1", output: "/k/Solo v1.epub" },
      not_written: [{ title: "Solo v2", output: "/k/Solo v2.epub" }],
    });
    render(<ExportStoppedPanel stopped={none} onReset={() => {}} />);
    expect(screen.getByText("No book was written.")).toBeTruthy();
    expect(screen.getByText("Not written: Solo v2.epub.")).toBeTruthy();
    expect(screen.queryByRole("button", { name: /Open folder/ })).toBeNull();
  });

  it("turns the backend outcome into results, or the error when a book failed", () => {
    const ok = { written: [stats("/k/a.epub")], failed: null, not_written: [] };
    expect(outcomeResults(ok)).toEqual([
      {
        output: "/k/a.epub",
        parts: [],
        pages: 6,
        skipped: 0,
        detectors_error: null,
        oversized: [],
      },
    ]);
    expect(() => outcomeResults({ ...ok, failed })).toThrow(
      "Solo v2 failed: No space left on device",
    );
  });
});
