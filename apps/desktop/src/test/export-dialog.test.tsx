import { fireEvent, render as mount, screen, waitFor } from "@testing-library/react";
import type { ReactElement } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ExportDialog } from "@/components/export-dialog";
import { TooltipProvider } from "@/components/ui/tooltip";
import { initialMode } from "@/lib/export-plan";
import { setBackend } from "@/lib/ipc";
import { ch, chapters, enabledButton, exportBackend, solo } from "./export-backend";

afterEach(() => setBackend(null));

const render = (ui: ReactElement) => mount(ui, { wrapper: TooltipProvider });

const dialog = (list = chapters, onExported?: () => void) => (
  <ExportDialog
    open
    onOpenChange={() => {}}
    picks={[{ manga: solo, chapters: list }]}
    onExported={onExported}
  />
);

describe("export dialog: bundles", () => {
  it("opens on By volume when the chapters span several volumes", () => {
    expect(initialMode(chapters)).toBe("by_volume");
    expect(initialMode([ch("1", "1"), ch("2", "1")])).toBe("single");
  });

  it("previews the files and exports every planned book", async () => {
    const { planVolumes, exportVolumes } = exportBackend();
    const onExported = vi.fn();
    render(dialog(chapters, onExported));
    expect(await screen.findByText("Solo v1.epub")).toBeTruthy();
    expect(screen.getByText("Solo v2.epub")).toBeTruthy();
    expect(screen.getByText("Chapters 1–2 · 2 chapters")).toBeTruthy();
    fireEvent.click(await enabledButton(/Export 2 books/));
    await waitFor(() => expect(exportVolumes).toHaveBeenCalledOnce());
    expect(exportVolumes.mock.calls[0]).toMatchObject([
      {
        grouping: { kind: "by_volume" },
        chapterIds: ["c1", "c2", "c3"],
        format: "epub",
        preset: expect.any(String),
      },
    ]);
    expect(await screen.findByText("Wrote 2 books")).toBeTruthy();
    expect(onExported).toHaveBeenCalledOnce();
    expect(planVolumes).toHaveBeenCalled();
  });

  it("writes one file per chapter", async () => {
    const { exportVolumes } = exportBackend();
    render(dialog());
    fireEvent.mouseDown(await screen.findByRole("tab", { name: "Per chapter" }));
    expect(await screen.findByText("3 files")).toBeTruthy();
    expect(screen.getByText("Solo ch 1.epub")).toBeTruthy();
    fireEvent.click(await enabledButton(/Export 3 books/));
    await waitFor(() => expect(exportVolumes).toHaveBeenCalledOnce());
    expect(exportVolumes.mock.calls[0][0].grouping).toEqual({ kind: "per_chapter" });
  });

  it("names a single book with the typed title", async () => {
    exportBackend();
    render(dialog());
    fireEvent.mouseDown(await screen.findByRole("tab", { name: "One book" }));
    const title = await screen.findByLabelText("Title");
    fireEvent.change(title, { target: { value: "Solo: omnibus" } });
    expect(await screen.findByText("Solo omnibus.epub")).toBeTruthy();
  });

  it("keeps the title field focused and replans once typing pauses", async () => {
    const { planVolumes } = exportBackend();
    render(dialog());
    fireEvent.mouseDown(await screen.findByRole("tab", { name: "One book" }));
    const title = await screen.findByLabelText("Title");
    title.focus();
    for (const value of ["C", "CO", "CON"]) fireEvent.change(title, { target: { value } });
    expect(await screen.findByText("CON.epub")).toBeTruthy();
    expect(screen.getByLabelText("Title")).toBe(title);
    expect(document.activeElement).toBe(title);
    const titles = planVolumes.mock.calls.map(([r]) => r.title);
    expect(titles).not.toContain("C");
    expect(titles).not.toContain("CO");
  });

  it("offers the title whenever the grouping makes one book", async () => {
    exportBackend();
    render(dialog([ch("1", "1"), ch("2", "1")]));
    fireEvent.mouseDown(await screen.findByRole("tab", { name: "By volume" }));
    expect(await screen.findByText("Solo v1.epub")).toBeTruthy();
    expect(screen.getByLabelText("Title")).toBeTruthy();
  });

  it("joins chapters from several series into one book", async () => {
    const exportChapters = vi.fn(async () => ({ output: "/k/x.epub", pages: 6, skipped: 0 }));
    exportBackend({ exportChapters });
    const other = { ...solo, id: "o", title: "Other" };
    render(
      <ExportDialog
        open
        onOpenChange={() => {}}
        picks={[
          { manga: solo, chapters: [ch("1", "1")] },
          { manga: other, chapters: [ch("9", null)] },
        ]}
      />,
    );
    expect(await screen.findByText("Solo +1 more.epub")).toBeTruthy();
    expect(screen.queryByRole("tab", { name: "By volume" })).toBeNull();
    fireEvent.click(await enabledButton(/Export book/));
    await waitFor(() => expect(exportChapters).toHaveBeenCalledOnce());
    expect(exportChapters.mock.calls[0]).toMatchObject([
      { chapter_dirs: ["/d/1", "/d/9"], output: expect.stringMatching(/Solo \+1 more\.epub$/) },
    ]);
  });
});
