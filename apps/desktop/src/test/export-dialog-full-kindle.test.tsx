import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
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
const full = "not enough space on Kindle at /media/me/Kindle";

afterEach(() => {
  setBackend(null);
  writeSettings({ kindleEmail: { ...readSettings().kindleEmail, to: "", host: "" } });
});

const dialog = () => (
  <TooltipProvider>
    <ExportDialog open onOpenChange={() => {}} picks={[{ manga: solo, chapters }]} />
  </TooltipProvider>
);

describe("export dialog: full Kindle", () => {
  it("shows the plain line and offers no Retry copy, but email still can", async () => {
    writeSettings({
      kindleEmail: {
        ...readSettings().kindleEmail,
        to: "reader@kindle.com",
        host: "smtp.example.test",
        username: "me@example.test",
      },
    });
    const sendUsb = vi.fn().mockRejectedValue(new Error(full));
    const sendEmail = vi.fn().mockResolvedValue({ to: "reader@kindle.com", bytes: 1 });
    exportBackend({ findKindles: async () => [kindle], sendUsb, sendEmail });
    render(dialog());
    // A format both routes take, so the book written for USB can be emailed as is.
    fireEvent.click(await screen.findByLabelText("PDF"));
    fireEvent.mouseDown(await screen.findByRole("tab", { name: "Kindle over USB" }));
    fireEvent.click(await enabledButton(/Export 2 books and copy/));
    expect(await screen.findByText(full)).toBeTruthy();
    expect(screen.queryByRole("button", { name: /Retry/ })).toBeNull();
    expect(screen.getByRole("button", { name: /Export 2 books and copy/ })).toHaveProperty(
      "disabled",
      true,
    );
    fireEvent.mouseDown(screen.getByRole("tab", { name: "Email to Kindle" }));
    fireEvent.click(await enabledButton(/^Retry email$/));
    await waitFor(() => expect(sendEmail).toHaveBeenCalled());
  });
});
