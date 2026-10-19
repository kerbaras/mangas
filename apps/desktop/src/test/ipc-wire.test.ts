import { describe, expect, it } from "vitest";
import { toDetectorsStatus, toExportArgs, toExportProgress } from "@/lib/ipc.wire";

describe("export events", () => {
  it("counts chapters from one and leaves page totals open", () => {
    expect(
      toExportProgress({ event: "export", kind: "chapter", index: 0, total: 3, path: "/a" }),
    ).toEqual({ stage: "Reading chapters", current: 1, total: 3 });
    expect(toExportProgress({ event: "export", kind: "pages", done: 41 })).toEqual({
      stage: "Cutting pages",
      current: 41,
      total: null,
    });
    expect(
      toExportProgress({ event: "model_download", id: "faces", done: 5, total: null }),
    ).toMatchObject({ current: 5, total: null });
    expect(toExportProgress({ event: "export", kind: "writing", pages: 9 })).toEqual({
      stage: "Writing file",
      current: 9,
      total: 9,
    });
  });
});

describe("export request", () => {
  it("renames to the camelCase Tauri arguments and drops an empty title", () => {
    expect(
      toExportArgs({
        chapter_dirs: ["/c/1"],
        device: "kindle-paperwhite12",
        format: "cbz",
        preset: "balanced",
        detectors: false,
        output: "/out",
      }),
    ).toEqual({
      chapterDirs: ["/c/1"],
      device: "kindle-paperwhite12",
      format: "cbz",
      preset: "balanced",
      detectors: false,
      output: "/out",
      replace: false,
    });
  });

  it("passes the replace choice", () => {
    const args = toExportArgs({
      chapter_dirs: [],
      device: "d",
      format: "cbz",
      preset: "balanced",
      detectors: false,
      output: "/o",
      replace: true,
    });
    expect(args.replace).toBe(true);
  });
});

describe("detector status", () => {
  it("falls back to cpu when no provider is reported", () => {
    expect(
      toDetectorsStatus({
        enabled: true,
        downloaded: false,
        runtime: { state: "not_downloaded", version: "1.28.3" },
        models: [],
        execution_provider: null,
        download_bytes: 5,
        disk_bytes: 1,
      }),
    ).toEqual({
      available: true,
      downloaded: false,
      provider: "cpu",
      runtime: { state: "not_downloaded", version: "1.28.3" },
      downloadBytes: 5,
      diskBytes: 1,
    });
  });
});
