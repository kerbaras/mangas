import { describe, expect, it } from "vitest";
import { toExportProgress } from "@/lib/ipc.wire";

describe("an export that has to download detector support first", () => {
  it("carries a notice the export hook shows as a toast", () => {
    const p = toExportProgress({ event: "detectors_missing", bytes: 24_300_000 });
    expect(p.notice).toMatch(/^Detector support was missing; downloading .+…$/);
    expect(p.stage).toBe(p.notice);
  });

  it("has no notice for ordinary progress", () => {
    const p = toExportProgress({ event: "model_download", id: "onnxruntime", done: 1, total: 2 });
    expect(p.notice).toBeUndefined();
  });
});
