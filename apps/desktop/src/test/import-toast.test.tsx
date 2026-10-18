import { act, fireEvent, render, screen } from "@testing-library/react";
import { Toaster } from "sonner";
import { describe, expect, it } from "vitest";
import { notifyImported } from "@/lib/import-toast";
import type { ImportReport } from "@/lib/ipc";

const report = (skipped: ImportReport["skipped"]) =>
  ({
    manga: {
      id: "berserk",
      source: "local",
      title: "Berserk v01",
      description: "",
      status: "unknown",
    },
    imported: [{ number: "1", title: null, pages: 2, path: "/data/local/berserk-v01/1" }],
    skipped,
  }) as unknown as ImportReport;

describe("import toast", () => {
  it("lists why entries and pages were skipped behind a toggle", async () => {
    render(<Toaster />);
    act(() =>
      notifyImported(
        report([
          { name: "Berserk v01.cbz/02.jpg", reason: "not an image" },
          { name: "notes.txt", reason: "not a chapter folder or .cbz file" },
        ]),
        "import",
      ),
    );

    expect(
      await screen.findByText("Imported 1 chapter (2 pages) of Berserk v01 · 2 skipped"),
    ).toBeInTheDocument();
    const toggle = screen.getByText("Why 2 items were skipped");
    fireEvent.click(toggle);
    expect(toggle.closest("details")).toHaveAttribute("open");
    expect(screen.getByText("Berserk v01.cbz/02.jpg")).toBeInTheDocument();
    expect(screen.getByText("— not an image")).toBeInTheDocument();
  });

  it("drops the list when a clean import replaces the toast", async () => {
    render(<Toaster />);
    act(() => notifyImported(report([{ name: "a.jpg", reason: "not an image" }]), "import"));
    expect(await screen.findByText("Why 1 item was skipped")).toBeInTheDocument();

    act(() => notifyImported(report([]), "import"));
    expect(
      await screen.findByText("Imported 1 chapter (2 pages) of Berserk v01"),
    ).toBeInTheDocument();
    expect(screen.queryByText(/skipped/)).not.toBeInTheDocument();
  });
});
