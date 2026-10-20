import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ExportDialog } from "@/components/export-dialog";
import { TooltipProvider } from "@/components/ui/tooltip";
import { setBackend } from "@/lib/ipc";
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

  it("says so without blocking a book that is only saved", async () => {
    const { exportVolumes } = exportBackend();
    exportVolumes.mockResolvedValueOnce([
      { output: book, parts: [], pages: 31, skipped: 0, oversized: [{ file: book, bytes: 5.1e7 }] },
    ]);
    dialog();
    fireEvent.click(await enabledButton(/^Export/));
    expect(await screen.findByText("Too big to email")).toBeTruthy();
    expect(screen.getByText(/Solo\.epub is 51\.0 MB/)).toBeTruthy();
  });
});
