import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { initialMode, VolumeExportDialog } from "@/components/volume-export-dialog";
import { type Backend, type Chapter, setBackend } from "@/lib/ipc";
import type { VolumeRequest } from "@/lib/ipc.volumes";
import { mockPlan } from "@/lib/volume-plan";

const ch = (number: string, volume: string | null): Chapter => ({
  id: `c${number}`,
  manga_id: "m",
  title: null,
  language: "en",
  volume,
  number,
  local_path: `/d/${number}`,
});

const chapters = [ch("1", "1"), ch("2", "1"), ch("3", "2")];
const manga = { source: "mangadex", id: "m", title: "Solo" };

afterEach(() => setBackend(null));

function backend() {
  const planVolumes = vi.fn(async (r: VolumeRequest) =>
    mockPlan(
      "Solo",
      chapters.filter((c) => r.chapterIds.includes(c.id)),
      r.grouping,
      r.title,
    ),
  );
  const exportVolumes = vi.fn(async () => [
    { output: "/k/Solo v1.epub", pages: 12, skipped: 0 },
    { output: "/k/Solo v2.epub", pages: 6, skipped: 0 },
  ]);
  setBackend({
    planVolumes,
    exportVolumes,
    listDevices: async () => [{ id: "kpw", name: "Paperwhite", width: 1236, height: 1648 }],
    detectorsStatus: async () => ({ ready: true, downloaded: false }),
    getSettings: async () => ({}),
    onExportProgress: async () => () => {},
  } as unknown as Backend);
  return { planVolumes, exportVolumes };
}

describe("export as volume", () => {
  it("opens on By volume when the chapters span several volumes", () => {
    expect(initialMode(chapters)).toBe("by_volume");
    expect(initialMode([ch("1", "1"), ch("2", "1")])).toBe("single");
  });

  it("previews the files and exports every planned book", async () => {
    const { planVolumes, exportVolumes } = backend();
    const onExported = vi.fn();
    render(
      <VolumeExportDialog
        open
        onOpenChange={() => {}}
        manga={manga}
        chapters={chapters}
        onExported={onExported}
      />,
    );
    expect(await screen.findByText("Solo v1.epub")).toBeTruthy();
    expect(screen.getByText("Solo v2.epub")).toBeTruthy();
    expect(screen.getByText("Chapters 1–2 · 2 chapters")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: /Export 2 books/ }));
    await waitFor(() => expect(exportVolumes).toHaveBeenCalledOnce());
    expect(exportVolumes.mock.calls[0]).toMatchObject([
      {
        grouping: { kind: "by_volume" },
        chapterIds: ["c1", "c2", "c3"],
        preset: expect.any(String),
      },
    ]);
    expect(await screen.findByText("Wrote 2 books")).toBeTruthy();
    expect(onExported).toHaveBeenCalledOnce();
    expect(planVolumes).toHaveBeenCalled();
  });

  it("names a single book with the typed title", async () => {
    backend();
    render(<VolumeExportDialog open onOpenChange={() => {}} manga={manga} chapters={chapters} />);
    fireEvent.mouseDown(await screen.findByRole("tab", { name: "One book" }));
    const title = await screen.findByLabelText("Title");
    fireEvent.change(title, { target: { value: "Solo: omnibus" } });
    expect(await screen.findByText("Solo omnibus.epub")).toBeTruthy();
  });

  it("keeps the title field focused and replans once typing pauses", async () => {
    const { planVolumes } = backend();
    render(<VolumeExportDialog open onOpenChange={() => {}} manga={manga} chapters={chapters} />);
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
    backend();
    const oneVolume = [ch("1", "1"), ch("2", "1")];
    render(<VolumeExportDialog open onOpenChange={() => {}} manga={manga} chapters={oneVolume} />);
    fireEvent.mouseDown(await screen.findByRole("tab", { name: "By volume" }));
    expect(await screen.findByText("Solo v1.epub")).toBeTruthy();
    expect(screen.getByLabelText("Title")).toBeTruthy();
  });
});
