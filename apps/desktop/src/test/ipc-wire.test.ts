import { describe, expect, it } from "vitest";
import { toDetectorsStatus, toExportArgs, toExportProgress } from "@/lib/ipc.wire";

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
