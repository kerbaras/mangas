import { renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { exportJob } from "@/lib/export-job";
import type { ExportPick } from "@/lib/export-name";
import { ExportStopped } from "@/lib/export-stopped";
import { useStorageChanged } from "@/lib/storage-changed";

const exportVolumes = vi.hoisted(() => vi.fn());
vi.mock("@/lib/ipc.volumes", () => ({ exportVolumes, exportFileName: vi.fn() }));

afterEach(() => exportVolumes.mockReset());

const pick = {
  manga: { source: "mangadex", id: "solo" },
  chapters: [{ id: "solo-1" }],
} as unknown as ExportPick;

const job = () =>
  exportJob({
    picks: [pick],
    mode: "every",
    size: 1,
    title: "",
    device: "kindle-paperwhite12",
    format: "epub",
    preset: "balanced",
    detectors: true,
    outputDir: null,
    replace: false,
    quality: null,
    splitMb: null,
  });

describe("export job", () => {
  it("reloads the detector status when a stopped run's kept books report a detector failure", async () => {
    const reload = vi.fn();
    renderHook(() => useStorageChanged(reload));
    const kept = {
      pages: 6,
      strips: 1,
      source_rows: 1,
      pagination_ms: 0,
      filters_ms: 0,
      skipped: 0,
      output: "/k/Solo v1.epub",
      detectors_error: "runtime download failed",
    };
    const failed = { title: "Solo v2", output: "/k/Solo v2.epub", reason: "No space left" };
    exportVolumes.mockRejectedValue(
      new ExportStopped({ written: [kept], failed, not_written: [] }),
    );
    await expect(job()()).rejects.toBeInstanceOf(ExportStopped);
    expect(reload).toHaveBeenCalledOnce();
  });
});
