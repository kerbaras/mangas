import { describe, expect, it } from "vitest";
import type { QueueEvent } from "@/lib/ipc.downloads";
import { mockQueue, putJob } from "@/lib/mock-queue";
import { makeJob } from "./job-fixture";

describe("browser mock queue", () => {
  it("reports pause all as a gate event and dismisses only finished jobs", async () => {
    const events: QueueEvent[] = [];
    await mockQueue.onQueueEvent((e) => events.push(e));
    putJob(makeJob({ id: 90, chapter_id: "x90", state: "failed", error: "HTTP 500" }));
    putJob(makeJob({ id: 91, chapter_id: "x91", state: "queued" }));

    await mockQueue.pauseDownload(null);
    expect(await mockQueue.queueStatus()).toEqual({ paused: true, held: false });
    expect(events).toContainEqual({ kind: "gate", paused: true, held: false });
    await mockQueue.resumeDownload(null);
    expect((await mockQueue.queueStatus()).paused).toBe(false);

    await expect(mockQueue.dismissDownload(91)).rejects.toThrow("pause or cancel it first");
    await mockQueue.dismissDownload(90);
    expect(events).toContainEqual({ kind: "removed", id: 90 });
    expect((await mockQueue.listDownloads()).some((j) => j.id === 90)).toBe(false);
  });
});
