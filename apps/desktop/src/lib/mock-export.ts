import type { ExportProgress } from "./ipc";

const stages = [
  "Reading pages",
  "Detecting cut points",
  "Cutting strips",
  "Filtering for e-ink",
  "Writing file",
];

const wait = (ms: number) => new Promise<void>((r) => setTimeout(r, ms));

/** Walks the export stages the Rust side reports, `total` steps each. */
export async function simulateExport(listeners: Set<(p: ExportProgress) => void>, total: number) {
  for (const stage of stages) {
    for (let current = 1; current <= total; current++) {
      await wait(60);
      for (const cb of listeners) cb({ stage, current, total });
    }
  }
}
