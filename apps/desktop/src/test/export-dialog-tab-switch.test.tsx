import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ExportDialog } from "@/components/export-dialog";
import { TooltipProvider } from "@/components/ui/tooltip";
import { setBackend } from "@/lib/ipc";
import { readSettings, writeSettings } from "@/lib/settings";
import { chapters, enabledButton, exportBackend, solo } from "./export-backend";

// #264: a delivery that finishes after the user picked another tab still shows its result.
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

const dialog = () => (
  <TooltipProvider>
    <ExportDialog
      open
      onOpenChange={() => {}}
      picks={[{ manga: solo, chapters: chapters.slice(0, 2) }]}
    />
  </TooltipProvider>
);

/** A backend call that resolves when the test says so. */
function held<T>() {
  let resolve: (value: T) => void = () => {};
  const fn = vi.fn(() => new Promise<T>((r) => (resolve = r)));
  return { fn, finish: (value: T) => act(async () => resolve(value)) };
}

describe("export dialog: delivery result after a tab switch", () => {
  it("keeps the Emailed line when the user picks Save to folder mid-send", async () => {
    const email = held<typeof sent>();
    exportBackend({ sendEmail: email.fn });
    render(dialog());
    fireEvent.mouseDown(await screen.findByRole("tab", { name: "Email to Kindle" }));
    fireEvent.click(await enabledButton(/Export book and email/));
    expect(await screen.findByRole("button", { name: "Sending email…" })).toBeTruthy();
    fireEvent.mouseDown(screen.getByRole("tab", { name: "Save to folder" }));
    await email.finish(sent);
    expect(await screen.findByText(/Emailed Solo v1\.epub .* to reader@kindle\.com/)).toBeTruthy();
    expect(screen.getByText("Exported 6 pages")).toBeTruthy();
  });

  it("keeps the Copied line when the user picks Email mid-copy", async () => {
    const copy = held<{ path: string; already_there: boolean }>();
    exportBackend({ findKindles: async () => [kindle], sendUsb: copy.fn });
    render(dialog());
    fireEvent.mouseDown(await screen.findByRole("tab", { name: "Kindle over USB" }));
    fireEvent.click(await enabledButton(/Export book and copy/));
    await vi.waitFor(() => expect(copy.fn).toHaveBeenCalledOnce());
    fireEvent.mouseDown(screen.getByRole("tab", { name: "Email to Kindle" }));
    await copy.finish({ path: "/media/me/Kindle/documents/Solo v1.epub", already_there: false });
    expect(await screen.findByText(/Copied to Kindle: .*Solo v1\.epub/)).toBeTruthy();
    expect(screen.getByText("Exported 6 pages")).toBeTruthy();
  });

  it("keeps the result when the tab changes while the book is still being written", async () => {
    const write = held<{ output: string; pages: number; skipped: number }[]>();
    const sendEmail = vi.fn().mockResolvedValue(sent);
    exportBackend({ exportVolumes: write.fn, sendEmail });
    render(dialog());
    fireEvent.mouseDown(await screen.findByRole("tab", { name: "Email to Kindle" }));
    fireEvent.click(await enabledButton(/Export book and email/));
    await vi.waitFor(() => expect(write.fn).toHaveBeenCalledOnce());
    fireEvent.mouseDown(screen.getByRole("tab", { name: "Save to folder" }));
    await write.finish([{ output: "/k/Solo v1.epub", pages: 6, skipped: 0 }]);
    expect(await screen.findByText(/Emailed Solo v1\.epub/)).toBeTruthy();
    expect(screen.getByText("Exported 6 pages")).toBeTruthy();
    expect(sendEmail).toHaveBeenCalledOnce();
  });

  it("still clears a finished result when the tab changes afterwards", async () => {
    exportBackend({ sendEmail: vi.fn().mockResolvedValue(sent) });
    render(dialog());
    fireEvent.mouseDown(await screen.findByRole("tab", { name: "Email to Kindle" }));
    fireEvent.click(await enabledButton(/Export book and email/));
    expect(await screen.findByText(/Emailed Solo v1\.epub/)).toBeTruthy();
    fireEvent.mouseDown(screen.getByRole("tab", { name: "Save to folder" }));
    expect(screen.queryByText(/Emailed Solo v1\.epub/)).toBeNull();
  });
});
