/** Volume export part of the IPC contract: plan a manga's books, then write them. */
import { backend, type Chapter, type ExportResult, type Preset } from "./ipc";

/** Mirrors `mangas_core::export::volume::Grouping` (serde tag = "kind", content = "size"). */
export type Grouping = { kind: "single" } | { kind: "by_volume" } | { kind: "every"; size: number };

/** Mirrors `mangas_core::export::volume::VolumePlan`: one EPUB to write. */
export type VolumePlan = {
  number: string | null;
  title: string;
  file_name: string;
  chapters: Chapter[];
};

export type VolumeRequest = {
  source: string;
  mangaId: string;
  chapterIds: string[];
  grouping: Grouping;
  /** Names a single book; ignored when the grouping makes several. */
  title?: string;
};

export type VolumeExport = VolumeRequest & {
  device: string;
  preset: Preset;
  detectors: boolean;
  outputDir: string;
};

export type VolumesBackend = {
  /** The books an export would write, in order, from the downloaded chapters among the ids. */
  planVolumes(request: VolumeRequest): Promise<VolumePlan[]>;
  /** Writes every planned book; resolves to one result per EPUB. */
  exportVolumes(request: VolumeExport): Promise<ExportResult[]>;
};

export const planVolumes = async (r: VolumeRequest) => (await backend()).planVolumes(r);
export const exportVolumes = async (r: VolumeExport) => (await backend()).exportVolumes(r);
