import { vi } from "vitest";
import {
  type Backend,
  type Chapter,
  type DetectorsStatus,
  type Manga,
  setBackend,
} from "@/lib/ipc";
import type { VolumeExport, VolumeRequest } from "@/lib/ipc.volumes";
import { mockPlan } from "@/lib/volume-plan";

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

const detectorsReady: DetectorsStatus = { available: true, downloaded: true, provider: "cpu" };

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
  const exportVolumes = vi.fn(async (r: VolumeExport) =>
    (await planVolumes(r)).map((p) => ({
      output: `/k/${p.file_name.replace(/\.epub$/, `.${r.format}`)}`,
      pages: 6,
      skipped: 0,
    })),
  );
  const freeExportNames = vi.fn(async (_dir: string, names: string[]) => names);
  setBackend({
    planVolumes,
    exportVolumes,
    freeExportNames,
    listDevices: async () => [
      { id: "kindle-paperwhite12", name: "Paperwhite", width: 1236, height: 1648, format: "epub" },
      { id: "kobo-clara", name: "Kobo Clara", width: 1072, height: 1448, format: "cbz" },
    ],
    detectorsStatus: async () => detectors,
    getSettings: async () => ({}),
    onExportProgress: async () => () => {},
    onCopyProgress: async () => () => {},
    ...overrides,
  } as unknown as Backend);
  return { planVolumes, exportVolumes };
}
