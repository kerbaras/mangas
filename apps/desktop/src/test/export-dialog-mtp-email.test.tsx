import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ExportDialog } from "@/components/export-dialog";
import { TooltipProvider } from "@/components/ui/tooltip";
import { setBackend } from "@/lib/ipc";
import { readSettings, writeSettings } from "@/lib/settings";
import { chapters, enabledButton, exportBackend, solo } from "./export-backend";

const book = "/k/Solo.epub";
const big = [
  {
    output: book,
    parts: [],
    pages: 31,
    skipped: 0,
    oversized: [{ file: book, bytes: 56_200_000 }],
  },
];

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

describe("export dialog: Kindle over MTP, then Email", () => {
  it("doesn't email a book over the email limit; it offers the split", async () => {
    const sendEmail = vi.fn();
    const { exportVolumes } = exportBackend({
      findKindles: vi.fn().mockResolvedValue([]),
      usbKindles: vi.fn().mockResolvedValue([{ name: "Kindle" }]),
      sendEmail,
    });
    exportVolumes.mockResolvedValue(big);
    render(
      <TooltipProvider>
        <ExportDialog open onOpenChange={() => {}} picks={[{ manga: solo, chapters }]} />
      </TooltipProvider>,
    );
    // A format both routes take, so the book written for USB can be emailed as is.
    fireEvent.click(await screen.findByLabelText("PDF"));
    fireEvent.mouseDown(await screen.findByRole("tab", { name: "Kindle over USB" }));
    fireEvent.click(await enabledButton(/^Export .*and copy/));
    expect(await screen.findByText("Kindle connected over MTP")).toBeTruthy();
    fireEvent.mouseDown(await screen.findByRole("tab", { name: "Email to Kindle" }));
    expect(await screen.findByText("Not emailed: too big for Send to Kindle")).toBeTruthy();
    expect(screen.queryByRole("button", { name: /Retry email/ })).toBeNull();
    fireEvent.click(await enabledButton(/^Export .*and email/));
    await waitFor(() => expect(exportVolumes).toHaveBeenCalledTimes(2));
    expect(sendEmail).not.toHaveBeenCalled();
    expect(screen.getByRole("button", { name: /Split into 45 MB parts/ })).toBeTruthy();
  });
});
