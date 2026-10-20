import { screen, waitFor } from "@testing-library/react";
import { expect, vi } from "vitest";
import { withExtension } from "@/lib/export-plan";
import {
  type Backend,
  type Chapter,
  type DetectorsStatus,
  type ExportFormat,
  type Manga,
  setBackend,
} from "@/lib/ipc";
import type { ExportResult } from "@/lib/ipc.export";
import type { VolumeExport, VolumeRequest } from "@/lib/ipc.volumes";
import { mockPlan, portable } from "@/lib/volume-plan";

export const ch = (number: string, volume: string | null): Chapter => ({
  id: `c${number}`,
  manga_id: "m",
  title: null,
  language: "en",
  volume,
  number,
  local_path: `/d/${number}`,
});

export const chapters = [ch("1", "1"), ch("2", "1"), ch("3", "2")];
export const solo: Manga = {
  source: "mangadex",
  id: "m",
  title: "Solo",
  cover_url: null,
  description: "",
  status: "ongoing",
};

/** A detector status: ready unless overridden. */
export const detectorState = (o: Partial<DetectorsStatus> = {}): DetectorsStatus => ({
  available: true,
  downloaded: true,
  provider: "cpu",
  runtime: { state: "downloaded", version: "1.28.3" },
  downloadBytes: 0,
  diskBytes: 190_000_000,
  ...o,
});

const detectorsReady = detectorState();

/** A backend for the export dialog: plans with the mock planner, writes one result per book. */
export function exportBackend(overrides: Partial<Backend> = {}, detectors = detectorsReady) {
  const planVolumes = vi.fn(async (r: VolumeRequest) =>
    mockPlan(
      "Solo",
      chapters.filter((c) => r.chapterIds.includes(c.id)),
      r.grouping,
      r.title,
    ),
  );
  const exportVolumes = vi.fn(
    async (r: VolumeExport): Promise<ExportResult[]> =>
      (await planVolumes(r)).map((p) => ({
        output: `/k/${p.file_name.replace(/\.epub$/, `.${r.format}`)}`,
        parts: [] as string[],
        pages: 6,
        skipped: 0,
      })),
  );
  const freeExportNames = vi.fn(async (_dir: string, names: string[]) => names);
  setBackend({
    planVolumes,
    exportVolumes,
    freeExportNames,
    exportFileName: async (title: string, format: ExportFormat) =>
      withExtension(portable(title), format),
    listDevices: async () => [
      { id: "kindle-paperwhite12", name: "Paperwhite", width: 1236, height: 1648, format: "epub" },
      { id: "kobo-clara", name: "Kobo Clara", width: 1072, height: 1448, format: "cbz" },
    ],
    detectorsStatus: async () => detectors,
    downloadDetectors: async () => detectorState(),
    cancelDetectorsDownload: async () => {},
    removeDetectors: async () => 0,
    onDetectorsProgress: async () => () => {},
    getSettings: async () => ({}),
    onExportProgress: async () => () => {},
    onCopyProgress: async () => () => {},
    ...overrides,
  } as unknown as Backend);
  return { planVolumes, exportVolumes };
}

/** The dialog's export button once it can be clicked (devices and detectors status loaded). */
export async function enabledButton(name: RegExp) {
  const button = await screen.findByRole("button", { name });
  await waitFor(() => expect(button).toHaveProperty("disabled", false));
  return button;
}
