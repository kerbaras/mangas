/** Volume export part of the IPC contract: plan a manga's books, then write them. */
import { backend, type Chapter, type ExportFormat, type ExportResult, type Preset } from "./ipc";

/** Mirrors `mangas_core::export::volume::Grouping` (serde tag = "kind", content = "size"). */
export type Grouping =
  | { kind: "single" }
  | { kind: "by_volume" }
  | { kind: "per_chapter" }
  | { kind: "every"; size: number };

/** Mirrors `mangas_core::export::volume::VolumePlan`: one book to write (named `.epub`). */
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
  format: ExportFormat;
  preset: Preset;
  detectors: boolean;
  outputDir: string | null;
  /** Write over books already in the folder instead of ` (2)` copies. */
  replace?: boolean;
};

export type VolumesBackend = {
  /** The books an export would write, in order, from the downloaded chapters among the ids. */
  planVolumes(request: VolumeRequest): Promise<VolumePlan[]>;
  /** Writes every planned book; resolves to one result per book. */
  exportVolumes(request: VolumeExport): Promise<ExportResult[]>;
  /** The names `names` would be written under in `dir` now: taken ones get ` (2)`, ` (3)`… */
  freeExportNames(dir: string, names: string[]): Promise<string[]>;
  /** The file name a book titled `title` is written under as `format` (shortened when too long). */
  exportFileName(title: string, format: ExportFormat): Promise<string>;
};

export const planVolumes = async (r: VolumeRequest) => (await backend()).planVolumes(r);
export const exportVolumes = async (r: VolumeExport) => (await backend()).exportVolumes(r);
export const freeExportNames = async (dir: string, names: string[]) =>
  (await backend()).freeExportNames(dir, names);
export const exportFileName = async (title: string, format: ExportFormat = "epub") =>
  (await backend()).exportFileName(title, format);
