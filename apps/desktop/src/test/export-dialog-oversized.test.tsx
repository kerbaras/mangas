import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ExportDialog } from "@/components/export-dialog";
import { TooltipProvider } from "@/components/ui/tooltip";
import { setBackend } from "@/lib/ipc";
import type { EstimateRequest } from "@/lib/ipc.volumes";
import { readSettings, writeSettings } from "@/lib/settings";
import { chapters, enabledButton, exportBackend, solo } from "./export-backend";

const sent = { to: "reader@kindle.com", bytes: 44_000_000 };
const book = "/k/Solo.epub";
const parts = [1, 2].map((n) => `/k/Solo (part ${n}).epub`);

beforeEach(() =>
  writeSettings({
    kindleEmail: {
      ...readSettings().kindleEmail,
      to: "reader@kindle.com",
      host: "smtp.example.test",
      username: "me@example.test",
    },
  }),
);
afterEach(() => {
  setBackend(null);
  writeSettings({ kindleEmail: { ...readSettings().kindleEmail, to: "", host: "" } });
});

const dialog = () =>
  render(
    <TooltipProvider>
      <ExportDialog
        open
        onOpenChange={() => {}}
        picks={[{ manga: solo, chapters: chapters.slice(0, 2) }]}
      />
    </TooltipProvider>,
  );

describe("export dialog: a written book over the email limit", () => {
  it("is not emailed, and the split it offers emails every part of the next export", async () => {
    const sendEmail = vi.fn().mockResolvedValue(sent);
    const { exportVolumes } = exportBackend({ sendEmail });
    const big = { file: book, bytes: 56_200_000 };
    exportVolumes
      .mockResolvedValueOnce([{ output: book, parts: [], pages: 31, skipped: 0, oversized: [big] }])
      .mockResolvedValueOnce([{ output: parts[0], parts, pages: 31, skipped: 0, oversized: [] }]);
    dialog();
    fireEvent.mouseDown(await screen.findByRole("tab", { name: "Email to Kindle" }));
    fireEvent.click(await enabledButton(/Export book and email/));
    expect(await screen.findByText("Not emailed: too big for Send to Kindle")).toBeTruthy();
    expect(screen.getByText(/Solo\.epub is 56\.2 MB; Send to Kindle by email takes up to 50/));
    expect(sendEmail).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: /Split into 45 MB parts/ }));
    const split = () => document.getElementById("export-split")?.getAttribute("aria-checked");
    await waitFor(() => expect(split()).toBe("true"));
    expect(screen.queryByText(/Not emailed/)).toBeNull();
    fireEvent.click(await enabledButton(/Export book and email/));
    await waitFor(() => expect(sendEmail).toHaveBeenCalledTimes(2));
    expect(exportVolumes.mock.calls[1][0].splitMb).toBe(45);
    expect(sendEmail.mock.calls).toEqual([
      [parts[0], "Solo (part 1 of 2)"],
      [parts[1], "Solo (part 2 of 2)"],
    ]);
  });

  it("holds a book that is only saved to the 200 MB web budget, not the email one", async () => {
    const { exportVolumes } = exportBackend();
    const written = (bytes: number) => [
      { output: book, parts: [], pages: 31, skipped: 0, oversized: [{ file: book, bytes }] },
    ];
    exportVolumes.mockResolvedValueOnce(written(5.1e7)).mockResolvedValueOnce(written(2.12e8));
    dialog();
    fireEvent.click(await enabledButton(/^Export/));
    await waitFor(() => expect(exportVolumes).toHaveBeenCalledTimes(1));
    const again = await enabledButton(/^Export/);
    expect(screen.queryByText(/Too big/)).toBeNull();
    fireEvent.click(again);
    expect(await screen.findByText("Too big for Send to Kindle")).toBeTruthy();
    expect(screen.getByText(/Solo\.epub is 212\.0 MB; Send to Kindle on the web takes up to 200/));
    fireEvent.click(screen.getByRole("button", { name: /Split into 180 MB parts/ }));
    const split = () => document.getElementById("export-split")?.getAttribute("aria-checked");
    await waitFor(() => expect(split()).toBe("true"));
  });

  it("offers the levers of the book's plan: the quality that fits, none when none does", async () => {
    const lever = (jpeg_quality: number | null) => ({
      split_bytes: 180_000_000,
      chapters_per_book: 1,
      jpeg_quality,
    });
    const fitLevers = vi.fn(async (r: EstimateRequest) => r.books.map(() => lever(80)));
    const { exportVolumes } = exportBackend({ fitLevers });
    const big = [{ file: book, bytes: 237_000_000 }];
    exportVolumes.mockResolvedValue([
      { output: book, parts: [], pages: 304, skipped: 0, oversized: big },
    ]);
    dialog();
    fireEvent.click(await enabledButton(/^Export/));
    const text =
      /To fit, split it into parts of 180 MB, or put 1 chapters in each book, or lower the JPEG quality to 80, then export again/;
    expect(await screen.findByText(text)).toBeTruthy();
    expect(fitLevers.mock.lastCall?.[0]).toMatchObject({ email: false, books: [["/d/1", "/d/2"]] });
    expect(screen.queryByText(/quality 70/)).toBeNull();

    fitLevers.mockImplementation(async (r) => r.books.map(() => lever(null)));
    fireEvent.click(await enabledButton(/^Export/));
    await waitFor(() => expect(exportVolumes).toHaveBeenCalledTimes(2));
    await waitFor(() => expect(fitLevers).toHaveBeenCalledTimes(2));
    const fix = await screen.findByText(/put 1 chapters in each book, then export again/);
    expect(fix.textContent).not.toMatch(/JPEG quality/);
  });
});
