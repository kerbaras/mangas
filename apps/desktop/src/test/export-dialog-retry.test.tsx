import { fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ExportDialog } from "@/components/export-dialog";
import { TooltipProvider } from "@/components/ui/tooltip";
import { setBackend } from "@/lib/ipc";
import { readSettings, writeSettings } from "@/lib/settings";
import { chapters, enabledButton, exportBackend, solo } from "./export-backend";

const kindle = {
  name: "Kindle",
  root: "/media/me/Kindle",
  documents: "/media/me/Kindle/documents",
};
const sent = { to: "reader@kindle.com", bytes: 1_000_000 };

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

const dialog = (onExported = vi.fn()) => (
  <TooltipProvider>
    <ExportDialog
      open
      onOpenChange={() => {}}
      picks={[{ manga: solo, chapters: chapters.slice(0, 2) }]}
      onExported={onExported}
    />
  </TooltipProvider>
);

describe("export dialog: Retry after a failed delivery", () => {
  it("emails the written book again instead of exporting it again", async () => {
    const sendEmail = vi
      .fn()
      .mockRejectedValueOnce(new Error("timed out after 60 s"))
      .mockRejectedValueOnce(new Error("connection refused"))
      .mockResolvedValueOnce(sent);
    const onExported = vi.fn();
    const { exportVolumes } = exportBackend({ sendEmail });
    render(dialog(onExported));
    fireEvent.mouseDown(await screen.findByRole("tab", { name: "Email to Kindle" }));
    fireEvent.click(await enabledButton(/Export book and email/));
    expect(await screen.findByText(/timed out/)).toBeTruthy();
    fireEvent.click(await enabledButton(/^Retry email$/));
    expect(await screen.findByText(/connection refused/)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: /^Retry$/ }));
    expect(await screen.findByText(/Emailed Solo v1\.epub/)).toBeTruthy();
    expect(exportVolumes).toHaveBeenCalledOnce();
    expect(onExported).toHaveBeenCalledOnce();
    expect(sendEmail.mock.calls).toEqual([
      ["/k/Solo v1.epub", "Solo v1"],
      ["/k/Solo v1.epub", "Solo v1"],
      ["/k/Solo v1.epub", "Solo v1"],
    ]);
    expect(screen.getByRole("button", { name: /Export book and email/ })).toBeTruthy();
  });

  it("copies only the books a failed USB copy left, without exporting again", async () => {
    const sendUsb = vi
      .fn()
      .mockResolvedValueOnce({
        path: "/media/me/Kindle/documents/Solo v1.epub",
        already_there: false,
      })
      .mockRejectedValueOnce(new Error("No space left on device"))
      .mockResolvedValueOnce({
        path: "/media/me/Kindle/documents/Solo v2.epub",
        already_there: false,
      });
    const { exportVolumes } = exportBackend({ findKindles: async () => [kindle], sendUsb });
    render(
      <TooltipProvider>
        <ExportDialog open onOpenChange={() => {}} picks={[{ manga: solo, chapters }]} />
      </TooltipProvider>,
    );
    fireEvent.mouseDown(await screen.findByRole("tab", { name: "Kindle over USB" }));
    fireEvent.click(await enabledButton(/Export 2 books and copy/));
    expect(await screen.findByText("Copy failed")).toBeTruthy();
    fireEvent.click(await enabledButton(/^Retry copy$/));
    expect(await screen.findByText(/Copied to Kindle: .*Solo v2\.epub/)).toBeTruthy();
    expect(screen.getByText(/Copied to Kindle: .*Solo v1\.epub/)).toBeTruthy();
    expect(exportVolumes).toHaveBeenCalledOnce();
    expect(sendUsb.mock.calls.map((c) => c[0])).toEqual([
      "/k/Solo v1.epub",
      "/k/Solo v2.epub",
      "/k/Solo v2.epub",
    ]);
  });

  it("copies the written book over USB when the user switches away from a failed email", async () => {
    const sendEmail = vi.fn().mockRejectedValueOnce(new Error("connection refused"));
    const sendUsb = vi.fn().mockResolvedValue({
      path: "/media/me/Kindle/documents/Solo v1.epub",
      already_there: false,
    });
    const { exportVolumes } = exportBackend({
      sendEmail,
      findKindles: async () => [kindle],
      sendUsb,
    });
    render(dialog());
    fireEvent.mouseDown(await screen.findByRole("tab", { name: "Email to Kindle" }));
    fireEvent.click(await enabledButton(/Export book and email/));
    expect(await screen.findByText(/connection refused/)).toBeTruthy();
    fireEvent.mouseDown(screen.getByRole("tab", { name: "Kindle over USB" }));
    fireEvent.click(await enabledButton(/^Retry copy$/));
    expect(await screen.findByText(/Copied to Kindle: .*Solo v1\.epub/)).toBeTruthy();
    expect(exportVolumes).toHaveBeenCalledOnce();
    expect(sendUsb.mock.calls.map((c) => c[0])).toEqual(["/k/Solo v1.epub"]);
  });

  it("offers no Retry once the user switches a failed email to Save to folder", async () => {
    const sendEmail = vi.fn().mockRejectedValueOnce(new Error("connection refused"));
    exportBackend({ sendEmail });
    render(dialog());
    fireEvent.mouseDown(await screen.findByRole("tab", { name: "Email to Kindle" }));
    fireEvent.click(await enabledButton(/Export book and email/));
    expect(await screen.findByText(/connection refused/)).toBeTruthy();
    expect(screen.getByRole("button", { name: /^Retry$/ })).toBeTruthy();
    fireEvent.mouseDown(screen.getByRole("tab", { name: "Save to folder" }));
    expect(screen.queryByRole("button", { name: /^Retry$/ })).toBeNull();
    expect(sendEmail).toHaveBeenCalledOnce();
  });

  it("exports again when the export itself failed", async () => {
    const exportVolumes = vi
      .fn()
      .mockRejectedValueOnce(new Error("permission denied: /k"))
      .mockResolvedValueOnce([{ output: "/k/Solo v1.epub", pages: 6, skipped: 0 }]);
    const sendEmail = vi.fn().mockResolvedValue(sent);
    exportBackend({ exportVolumes, sendEmail });
    render(dialog());
    fireEvent.mouseDown(await screen.findByRole("tab", { name: "Email to Kindle" }));
    fireEvent.click(await enabledButton(/Export book and email/));
    expect(await screen.findByText(/permission denied/)).toBeTruthy();
    expect(sendEmail).not.toHaveBeenCalled();
    fireEvent.click(await enabledButton(/Export book and email/));
    expect(await screen.findByText(/Emailed Solo v1\.epub/)).toBeTruthy();
    expect(exportVolumes).toHaveBeenCalledTimes(2);
    expect(sendEmail).toHaveBeenCalledOnce();
  });
});
