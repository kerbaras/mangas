import { describe, expect, it } from "vitest";
import {
  toDetectorsStatus,
  toDownloadProgress,
  toExportArgs,
  toExportProgress,
} from "@/lib/ipc.wire";

describe("download events", () => {
  it("maps a page to running progress", () => {
    expect(toDownloadProgress({ kind: "page_done", chapter: "c1", page: 3, total: 20 })).toEqual({
      chapter_id: "c1",
      page: 3,
      pages: 20,
      done: false,
      error: null,
    });
  });
  it("maps chapter_done and error to finished progress", () => {
    expect(
      toDownloadProgress({ kind: "chapter_done", chapter: "c1", path: "/x", pages: 20 }),
    ).toMatchObject({ done: true, page: 20, pages: 20, error: null });
    expect(toDownloadProgress({ kind: "error", chapter: "c1", message: "429" })).toMatchObject({
      done: true,
      error: "429",
    });
  });
});

describe("kindle events", () => {
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
  });
});

describe("export request", () => {
  it("renames to the camelCase Tauri arguments and drops an empty title", () => {
    expect(
      toExportArgs({
        chapter_dirs: ["/c/1"],
        device: "kindle-paperwhite12",
        preset: "balanced",
        detectors: false,
        output: "/out",
      }),
    ).toEqual({
      chapterDirs: ["/c/1"],
      device: "kindle-paperwhite12",
      preset: "balanced",
      detectors: false,
      output: "/out",
    });
  });
});

describe("detector status", () => {
  it("falls back to cpu when no provider is reported", () => {
    expect(
      toDetectorsStatus({ enabled: true, downloaded: false, models: [], execution_provider: null }),
    ).toEqual({ available: true, downloaded: false, provider: "cpu" });
  });
});
