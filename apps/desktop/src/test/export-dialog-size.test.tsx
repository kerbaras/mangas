import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ExportDialog } from "@/components/export-dialog";
import { TooltipProvider } from "@/components/ui/tooltip";
import { setBackend } from "@/lib/ipc";
import type { EstimateRequest } from "@/lib/ipc.volumes";
import { readSettings, writeSettings } from "@/lib/settings";
import { chapters, enabledButton, exportBackend, solo } from "./export-backend";

// The plan (300 ms) and estimate (600 ms) debounces wait in real time and would spend most of
// `findByText`'s 1 s on the clock; this file checks budgets, not pauses (#408).
vi.mock("@/hooks/use-debounced", () => ({ useDebounced: <T,>(value: T) => value }));

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

// 80 MB a chapter at the device's quality, 40 MB at 50: three chapters are 240 MB.
const MB = 1_000_000;
const estimateExport = vi.fn(async (r: EstimateRequest) =>
  r.books.map((b) => {
    const bytes = b.length * (r.quality === 50 ? 40 : 80) * MB;
    const budget = (r.email ? 50 : 200) * MB;
    const split = r.splitMb ? r.splitMb * MB : bytes;
    const over = Math.min(bytes, split) > budget;
    const levers = { split_bytes: 180 * MB, chapters_per_book: 2, jpeg_quality: 50 };
    return {
      pages: b.length * 60,
      estimated_bytes: bytes,
      budget_bytes: budget,
      over_budget: over,
      jpeg_quality: r.quality ?? 90,
      files: Math.ceil(bytes / split),
      levers: over ? levers : null,
    };
  }),
);
const OVER = "About 240 MB of 200 MB, 180 pages, JPEG quality 90";

const openOneBook = async () => {
  render(dialog());
  fireEvent.mouseDown(await screen.findByRole("tab", { name: "One book" }));
  expect(await screen.findByText(OVER)).toBeTruthy();
};

describe("export dialog: Send-to-Kindle size budget", () => {
  it("holds a saved book to 200 MB and shows it against the budget in the preview", async () => {
    exportBackend({ estimateExport });
    await openOneBook();
    expect(estimateExport.mock.lastCall?.[0]).toMatchObject({
      books: [["/d/1", "/d/2", "/d/3"]],
      email: false,
      splitMb: null,
    });
    expect(screen.getByRole("alert").textContent).toMatch(/over the 200 MB Send to Kindle web/);
    expect(screen.getByText("240 MB of 200 MB")).toBeTruthy();
  });

  it("splits into parts only when that lever is picked, and exports with it", async () => {
    const { exportVolumes } = exportBackend({ estimateExport });
    await openOneBook();
    fireEvent.click(screen.getByRole("button", { name: /Split into 180 MB parts/ }));
    expect(
      await screen.findByText("About 240 MB in 2 parts of 200 MB, 180 pages, JPEG quality 90"),
    ).toBeTruthy();
    expect(screen.queryByRole("alert")).toBeNull();
    expect(estimateExport.mock.lastCall?.[0].splitMb).toBe(180);

    exportVolumes.mockResolvedValueOnce([
      {
        output: "/k/Solo (part 1).epub",
        parts: ["/k/Solo (part 1).epub", "/k/Solo (part 2).epub"],
        pages: 180,
        skipped: 0,
        jpegQuality: 90,
      },
    ]);
    fireEvent.click(await enabledButton(/Export book/));
    await waitFor(() => expect(exportVolumes).toHaveBeenCalledOnce());
    expect(exportVolumes.mock.calls[0][0]).toMatchObject({ splitMb: 180, quality: null });
    expect(await screen.findByText("JPEG quality 90")).toBeTruthy();
  });

  it("lowers the JPEG quality only when that lever is picked, and keeps it for the device", async () => {
    const { exportVolumes } = exportBackend({ estimateExport });
    await openOneBook();
    fireEvent.click(screen.getByRole("button", { name: /JPEG quality 50/ }));
    expect(
      await screen.findByText("About 120 MB of 200 MB, 180 pages, JPEG quality 50"),
    ).toBeTruthy();
    expect(screen.queryByRole("alert")).toBeNull();
    expect(readSettings().jpegQuality).toEqual({ "kindle-paperwhite12": 50 });
    fireEvent.click(await enabledButton(/Export book/));
    await waitFor(() => expect(exportVolumes).toHaveBeenCalledOnce());
    expect(exportVolumes.mock.calls[0][0]).toMatchObject({ quality: 50, splitMb: null });
  });

  it("writes fewer chapters per book when that lever is picked", async () => {
    exportBackend({ estimateExport });
    await openOneBook();
    fireEvent.click(screen.getByRole("button", { name: /2 chapters per book/ }));
    await waitFor(() =>
      expect(estimateExport.mock.lastCall?.[0].books).toEqual([["/d/1", "/d/2"], ["/d/3"]]),
    );
    expect(screen.getByRole("tab", { name: "Every N" }).getAttribute("aria-selected")).toBe("true");
  });

  it("starts at the quality stored for the device", async () => {
    await writeSettings({ jpegQuality: { "kindle-paperwhite12": 50 } });
    exportBackend({ estimateExport });
    render(dialog());
    fireEvent.mouseDown(await screen.findByRole("tab", { name: "One book" }));
    expect(
      await screen.findByText("About 120 MB of 200 MB, 180 pages, JPEG quality 50"),
    ).toBeTruthy();
    expect(screen.getByRole("combobox", { name: "JPEG quality" }).textContent).toBe(
      "JPEG quality: 50",
    );
  });
});
