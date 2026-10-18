import { describe, expect, it } from "vitest";
import { enqueueSummary } from "@/lib/enqueue-summary";
import { mockBackend } from "@/lib/ipc.mock";

describe("enqueueSummary", () => {
  it("says nothing when nothing was skipped", () => {
    expect(enqueueSummary({ queued: 3, missing: [], on_disk: [] })).toBeNull();
  });

  it("counts queued and skipped chapters", () => {
    expect(enqueueSummary({ queued: 3, missing: [], on_disk: ["a", "b"] })).toBe(
      "Queued 3 chapters, skipped 2 chapters already on disk",
    );
    expect(enqueueSummary({ queued: 1, missing: [], on_disk: ["a"] })).toBe(
      "Queued 1 chapter, skipped 1 chapter already on disk",
    );
  });

  it("reports a request where everything was on disk", () => {
    expect(enqueueSummary({ queued: 0, missing: [], on_disk: ["a"] })).toBe(
      "Skipped 1 chapter already on disk",
    );
  });
});

describe("mock downloadChapters", () => {
  it("skips chapters on disk unless forced", async () => {
    const [manga] = await mockBackend.listLibrary();
    if (!manga) throw new Error("no sample manga");
    const chapters = await mockBackend.listChapters(manga.source, manga.id);
    const onDisk = chapters.find((c) => c.local_path);
    if (!onDisk) throw new Error("no sample chapter on disk");
    const skipped = await mockBackend.downloadChapters(manga.source, manga.id, [onDisk.id]);
    expect(skipped).toEqual({ queued: 0, missing: [], on_disk: [onDisk.id] });
    const forced = await mockBackend.downloadChapters(manga.source, manga.id, [onDisk.id], true);
    expect(forced).toEqual({ queued: 1, missing: [], on_disk: [] });
  });
});
