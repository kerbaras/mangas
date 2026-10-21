import { fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ExportDialog } from "@/components/export-dialog";
import { TooltipProvider } from "@/components/ui/tooltip";
import { ExportStopped } from "@/lib/export-stopped";
import { setBackend } from "@/lib/ipc";
import type { VolumePlan, VolumeRequest } from "@/lib/ipc.volumes";
import { writeSettings } from "@/lib/settings";
import { ch, enabledButton, exportBackend, solo } from "./export-backend";

afterEach(() => {
  setBackend(null);
  writeSettings({ replaceExisting: false });
});

const [c0, c1, c3] = [ch("0", "0"), ch("1", "1"), ch("3", "3")];
const dialog = (picked: (typeof c0)[]) => (
  <TooltipProvider>
    <ExportDialog open onOpenChange={() => {}} picks={[{ manga: solo, chapters: picked }]} />
  </TooltipProvider>
);
/** `c0`, a release downloaded after the stop, shares `c1`'s title: both plan as `Solo ch 1.epub`. */
const planVolumes = vi.fn(
  async (r: VolumeRequest): Promise<VolumePlan[]> =>
    [c0, c1, c3]
      .filter((c) => r.chapterIds.includes(c.id))
      .map((c) => {
        const title = c === c3 ? "Solo ch 3" : "Solo ch 1";
        return { number: c.volume, title, file_name: `${title}.epub`, chapters: [c] };
      }),
);
const stats = { pages: 6, strips: 0, source_rows: 0, pagination_ms: 0, filters_ms: 0, skipped: 0 };
const result = (title: string) => ({ output: `/k/${title}.epub`, parts: [], pages: 6, skipped: 0 });
const previewed = () =>
  screen
    .getAllByText(/^Solo ch \d( \(\d\))?\.epub$/)
    .filter((e) => e.classList.contains("font-mono")) // the book list, not the stopped panel
    .map((e) => e.textContent);

describe("Try again preview with Replace on (#337)", () => {
  it("names a same-titled newcomer as the retry writes it, around the kept file", async () => {
    writeSettings({ replaceExisting: true });
    const exportVolumes = vi
      .fn()
      .mockRejectedValueOnce(
        new ExportStopped({
          written: [{ ...stats, output: "/k/Solo ch 1.epub", chapters: ["c1"] }],
          failed: { title: "Solo ch 3", output: "/k/Solo ch 3.epub", reason: "io: disk full" },
          not_written: [],
        }),
      )
      .mockResolvedValueOnce([result("Solo ch 1 (2)"), result("Solo ch 3")]);
    exportBackend({ planVolumes, exportVolumes });
    const { rerender } = render(dialog([c1, c3]));
    fireEvent.click(await enabledButton(/^Export 2 books/));
    expect(await screen.findByText("Export stopped")).toBeTruthy();
    rerender(dialog([c0, c1, c3]));
    // Planned ahead of the kept book, the newcomer is the one numbered: `ch 1.epub` is kept.
    expect(await screen.findByText("Solo ch 1 (2).epub")).toBeTruthy();
    expect(previewed()).toEqual(["Solo ch 1 (2).epub", "Solo ch 1.epub", "Solo ch 3.epub"]);
    fireEvent.click(await enabledButton(/^Try again$/));
    expect(
      await screen.findByText(/Already there from the last try: Solo ch 1\.epub/),
    ).toBeTruthy();
    expect(exportVolumes.mock.calls[1][0]).toMatchObject({
      kept: [{ chapters: ["c1"], file: "Solo ch 1.epub" }],
      replace: true,
    });
    expect(screen.getByText("Solo ch 1 (2).epub")).toBeTruthy();
  });
});
