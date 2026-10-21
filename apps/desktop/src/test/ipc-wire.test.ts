import { describe, expect, it } from "vitest";
import { toDetectorsStatus, toExportArgs, toExportProgress, toExportResult } from "@/lib/ipc.wire";

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
      quality: null,
      splitMb: null,
      direction: null,
    });
  });

  it("passes the JPEG quality and part size", () => {
    const args = toExportArgs({
      chapter_dirs: [],
      device: "d",
      format: "epub",
      preset: "balanced",
      detectors: false,
      output: "/o",
      quality: 60,
      splitMb: 45,
    });
    expect(args).toMatchObject({ quality: 60, splitMb: 45 });
  });

  it("passes the page turn", () => {
    const args = toExportArgs({
      chapter_dirs: [],
      device: "d",
      format: "epub",
      preset: "balanced",
      detectors: false,
      output: "/o",
      direction: "rtl",
    });
    expect(args).toMatchObject({ direction: "rtl" });
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

describe("export stats", () => {
  it("passes on the files written over the email limit, measured on disk", () => {
    const stats = { pages: 31, strips: 0, source_rows: 0, pagination_ms: 0, filters_ms: 0 };
    const big = { path: "/k/Solo.epub", bytes: 56_200_000 };
    expect(
      toExportResult({ ...stats, skipped: 0, output: "/k/Solo.epub", oversized: [big] }).oversized,
    ).toEqual([{ file: "/k/Solo.epub", bytes: 56_200_000 }]);
    expect(toExportResult({ ...stats, skipped: 0, output: "/k/a.epub" }).oversized).toEqual([]);
  });
});
