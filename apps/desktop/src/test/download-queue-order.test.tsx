import { act, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { DownloadsProvider, useDownloads } from "@/hooks/use-downloads";
import { type Backend, setBackend } from "@/lib/ipc";
import type { DownloadJob, QueueEvent } from "@/lib/ipc.downloads";
import { mockBackend } from "@/lib/ipc.mock";
import { sampleCatalog } from "@/lib/mock-data";

vi.mock("@/lib/notify", () => ({ notifyError: vi.fn() }));

afterEach(() => setBackend(null));

const job = (state: DownloadJob["state"], updated_at: number): DownloadJob => ({
  ...({} as DownloadJob),
  id: 1,
  source: "mangadex",
  manga_id: "m",
  chapter_id: "c1",
  state,
  error: null,
  pages_done: 0,
  pages_total: 0,
  manga_title: "Kaiju",
  number: "1",
  updated_at,
});

function State() {
  return <p data-testid="state">{useDownloads().get("c1")?.state ?? "none"}</p>;
}

describe("DownloadsProvider", () => {
  it("keeps an event over the startup snapshot that resolves after it", async () => {
    let emit: (e: QueueEvent) => void = () => {};
    let resolveList: (jobs: DownloadJob[]) => void = () => {};
    setBackend({
      onQueueEvent: async (cb: (e: QueueEvent) => void) => {
        emit = cb;
        return () => {};
      },
      listDownloads: () => new Promise<DownloadJob[]>((r) => (resolveList = r)),
      reportError: async () => {},
    } as unknown as Backend);
    render(
      <DownloadsProvider>
        <State />
      </DownloadsProvider>,
    );
    await act(async () => {});
    // Same millisecond as the snapshot: the event still wins.
    act(() => emit({ kind: "paused", job: job("paused", 10) }));
    await act(async () => resolveList([job("running", 10)]));
    expect(screen.getByTestId("state").textContent).toBe("paused");
  });
});

describe("browser mock queue", () => {
  it("stops a simulated download once it is paused", async () => {
    const manga = sampleCatalog[0];
    const chapters = await mockBackend.listChapters(manga.source, manga.id);
    const target = chapters.find((c) => !c.local_path && c.availability?.kind !== "locked");
    if (!target) throw new Error("no downloadable sample chapter");
    await mockBackend.downloadChapters(manga.source, manga.id, [target.id]);
    await vi.waitFor(async () => {
      const [j] = await mockBackend.listDownloads();
      expect(j?.state).toBe("running");
    });
    await mockBackend.pauseDownload(null);
    await new Promise((r) => setTimeout(r, 700));
    const [paused] = await mockBackend.listDownloads();
    expect(paused.state).toBe("paused");
    expect(target.local_path).toBeNull();
  });
});
