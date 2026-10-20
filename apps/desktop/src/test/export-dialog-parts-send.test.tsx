import { fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ExportDialog } from "@/components/export-dialog";
import { TooltipProvider } from "@/components/ui/tooltip";
import { setBackend } from "@/lib/ipc";
import { readSettings, writeSettings } from "@/lib/settings";
import { chapters, enabledButton, exportBackend, solo } from "./export-backend";

const sent = { to: "reader@kindle.com", bytes: 44_000_000 };
const parts = [1, 2, 3].map((n) => `/k/Solo (part ${n}).epub`);

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

describe("export dialog: emailing a book split into parts", () => {
  it("sends each part as part n of 3 and names the parts a refusal left unsent", async () => {
    const sendEmail = vi
      .fn()
      .mockResolvedValueOnce(sent)
      .mockRejectedValueOnce(new Error("permanent error (554): refused"))
      .mockResolvedValue(sent);
    const { exportVolumes } = exportBackend({ sendEmail });
    exportVolumes.mockResolvedValueOnce([{ output: parts[0], parts, pages: 123, skipped: 0 }]);
    render(
      <TooltipProvider>
        <ExportDialog
          open
          onOpenChange={() => {}}
          picks={[{ manga: solo, chapters: chapters.slice(0, 2) }]}
        />
      </TooltipProvider>,
    );
    fireEvent.mouseDown(await screen.findByRole("tab", { name: "Email to Kindle" }));
    fireEvent.click(await enabledButton(/Export book and email/));
    expect(await screen.findByText(/554/)).toBeTruthy();
    expect(screen.getByText(/Emailed Solo \(part 1\)\.epub/)).toBeTruthy();
    expect(screen.getByText("Not sent: Solo (part 2).epub, Solo (part 3).epub")).toBeTruthy();
    fireEvent.click(await enabledButton(/^Retry email$/));
    expect(await screen.findByText(/Emailed Solo \(part 3\)\.epub/)).toBeTruthy();
    expect(sendEmail.mock.calls).toEqual([
      [parts[0], "Solo (part 1 of 3)"],
      [parts[1], "Solo (part 2 of 3)"],
      [parts[1], "Solo (part 2 of 3)"],
      [parts[2], "Solo (part 3 of 3)"],
    ]);
  });
});
