import { fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ExportDialog } from "@/components/export-dialog";
import { TooltipProvider } from "@/components/ui/tooltip";
import { setBackend } from "@/lib/ipc";
import { readSettings, writeSettings } from "@/lib/settings";
import { chapters, enabledButton, exportBackend, solo } from "./export-backend";

// #260: "Try again" after a failed export exports again with the same options, in one click.
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

const dialog = () => (
  <TooltipProvider>
    <ExportDialog
      open
      onOpenChange={() => {}}
      picks={[{ manga: solo, chapters: chapters.slice(0, 2) }]}
    />
  </TooltipProvider>
);

const failingOnce = () =>
  vi
    .fn()
    .mockRejectedValueOnce(new Error("permission denied: /k"))
    .mockResolvedValueOnce([{ output: "/k/Solo v1.epub", parts: [], pages: 6, skipped: 0 }]);

describe("export dialog: Try again after a failed export", () => {
  it("exports again with the same options", async () => {
    const exportVolumes = failingOnce();
    exportBackend({ exportVolumes });
    render(dialog());
    fireEvent.click(await screen.findByRole("switch", { name: /Replace existing files/ }));
    fireEvent.click(await enabledButton(/^Export book$/));
    expect(await screen.findByText(/permission denied/)).toBeTruthy();
    fireEvent.click(await enabledButton(/^Try again$/));
    expect(await screen.findByText("Exported 6 pages")).toBeTruthy();
    expect(exportVolumes).toHaveBeenCalledTimes(2);
    const [first, second] = exportVolumes.mock.calls.map((c) => c[0]);
    expect(second).toEqual(first);
    expect(second.replace).toBe(true);
  });

  it("delivers the book too when the failed export was going to be emailed", async () => {
    const exportVolumes = failingOnce();
    const sendEmail = vi.fn().mockResolvedValue({ to: "reader@kindle.com", bytes: 1_000_000 });
    exportBackend({ exportVolumes, sendEmail });
    render(dialog());
    fireEvent.mouseDown(await screen.findByRole("tab", { name: "Email to Kindle" }));
    fireEvent.click(await enabledButton(/Export book and email/));
    expect(await screen.findByText(/permission denied/)).toBeTruthy();
    fireEvent.click(await enabledButton(/^Try again$/));
    expect(await screen.findByText(/Emailed Solo v1\.epub/)).toBeTruthy();
    expect(exportVolumes).toHaveBeenCalledTimes(2);
    expect(sendEmail).toHaveBeenCalledOnce();
  });

  it("shows the new failure when the second export fails too", async () => {
    const exportVolumes = vi
      .fn()
      .mockRejectedValueOnce(new Error("permission denied: /k"))
      .mockRejectedValueOnce(new Error("no space left on device"));
    exportBackend({ exportVolumes });
    render(dialog());
    fireEvent.click(await enabledButton(/^Export book$/));
    expect(await screen.findByText(/permission denied/)).toBeTruthy();
    fireEvent.click(await enabledButton(/^Try again$/));
    expect(await screen.findByText(/no space left on device/)).toBeTruthy();
    expect(screen.getByRole("button", { name: /^Try again$/ })).toBeTruthy();
    expect(exportVolumes).toHaveBeenCalledTimes(2);
  });
});
