/**
 * Browser mock of detector support. `?no-detectors`: a build without it; `?detectors-missing`:
 * nothing downloaded yet; `?detectors-fail`: every download fails (Retry fails again).
 */
import type { DetectorsBackend, DetectorsProgress, DetectorsStatus } from "./ipc.detectors";
import { mockFlags } from "./mock-flags";

const wait = (ms: number) => new Promise<void>((r) => setTimeout(r, ms));
const VERSION = "1.28.3";
const ITEMS: [string, number][] = [
  ["onnxruntime", 9_130_098],
  ["comic-text-and-bubble-detector", 168_481_531],
  ["anime-face-detect-v0-n", 12_102_558],
];
const TOTAL = ITEMS.reduce((sum, [, size]) => sum + size, 0);

export function mockDetectors(): DetectorsBackend {
  let downloaded = !mockFlags.detectorsMissing;
  let cancelled = false;
  const listeners = new Set<(p: DetectorsProgress) => void>();
  const status = (): DetectorsStatus => ({
    available: !mockFlags.noDetectors,
    downloaded,
    provider: "cpu",
    runtime: mockFlags.noDetectors
      ? { state: "unavailable" }
      : { state: downloaded ? "downloaded" : "not_downloaded", version: VERSION },
    downloadBytes: downloaded ? 0 : TOTAL,
    diskBytes: downloaded ? TOTAL : 0,
  });
  return {
    async detectorsStatus() {
      await wait(120);
      return status();
    },
    async downloadDetectors() {
      cancelled = false;
      for (const [item, total] of ITEMS) {
        for (let step = 1; step <= 8; step += 1) {
          await wait(120);
          if (cancelled) throw new Error("download cancelled");
          if (mockFlags.detectorsFail && item !== "onnxruntime")
            throw new Error(`network: error sending request for url (${item})`);
          for (const cb of listeners) cb({ item, done: (total * step) / 8, total });
        }
      }
      downloaded = true;
      return status();
    },
    async cancelDetectorsDownload() {
      cancelled = true;
    },
    async removeDetectors() {
      const freed = downloaded ? TOTAL : 0;
      downloaded = false;
      return freed;
    },
    async onDetectorsProgress(cb) {
      listeners.add(cb);
      return () => listeners.delete(cb);
    },
  };
}
