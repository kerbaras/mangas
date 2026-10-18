import { afterEach, describe, expect, it, vi } from "vitest";
import { type Backend, setBackend } from "@/lib/ipc";
import { summarise, summaryLine, type UpdateRun } from "@/lib/ipc.updates";
import { intervalMs, readUpdates, runCheck } from "@/lib/updates";

const chapter = (id: string, number: string) =>
  ({
    id,
    number,
    manga_id: "m",
    title: null,
    language: "en",
    volume: null,
    local_path: null,
  }) as const;

describe("update summary", () => {
  it("words a run like the backend", () => {
    expect(summaryLine({ chapters: 0, titles: 0, unreachable: 0 })).toBe("Up to date");
    expect(summaryLine({ chapters: 12, titles: 3, unreachable: 0 })).toBe(
      "12 new chapters in 3 titles",
    );
    expect(summaryLine({ chapters: 0, titles: 0, unreachable: 2 })).toBe("2 providers unreachable");
    expect(summaryLine({ chapters: 1, titles: 1, unreachable: 1 })).toBe(
      "1 new chapter in 1 title · 1 provider unreachable",
    );
  });

  it("counts numbers once and providers once", () => {
    const update = (chapters: ReturnType<typeof chapter>[]) => ({
      source: "mangadex",
      manga_id: "m",
      title: "M",
      auto_download: false,
      chapters: [...chapters],
    });
    const failure = (source: string) => ({
      source,
      manga_id: "x",
      title: "X",
      error: "down",
      challenge: false,
    });
    const s = summarise(
      [update([chapter("a", "12"), chapter("b", "12.0")]), update([chapter("c", "3")])],
      [failure("comick"), failure("comick"), failure("weebcentral")],
    );
    expect(s).toEqual({ chapters: 2, titles: 2, unreachable: 2 });
  });
});

describe("update runs", () => {
  afterEach(() => setBackend(null));

  it("joins the run in flight and counts runs that found something", async () => {
    let finish: (run: UpdateRun) => void = () => {};
    const checkUpdates = vi.fn(() => new Promise<UpdateRun>((r) => (finish = r)));
    setBackend({ checkUpdates } as unknown as Backend);
    const before = readUpdates().found;
    const first = runCheck(true);
    const second = runCheck(false);
    await Promise.resolve();
    expect(readUpdates().checking).toBe(true);
    const run: UpdateRun = {
      updates: [
        {
          source: "s",
          manga_id: "m",
          title: "M",
          auto_download: false,
          chapters: [chapter("a", "1")],
        },
      ],
      failures: [],
      checked: 1,
      skipped: 0,
      summary: { chapters: 1, titles: 1, unreachable: 0 },
    };
    finish(run);
    expect(await first).toBe(run);
    expect(await second).toBe(run);
    expect(checkUpdates).toHaveBeenCalledTimes(1);
    expect(checkUpdates).toHaveBeenCalledWith(true);
    expect(readUpdates()).toMatchObject({ checking: false, found: before + 1, last: run });
  });

  it("turns the hours setting into a timer, 0 meaning off", () => {
    expect(intervalMs(6)).toBe(21_600_000);
    expect(intervalMs(0)).toBeNull();
    expect(intervalMs(Number.NaN)).toBeNull();
  });
});
