/** Volume export part of the IPC contract: plan a manga's books, then write them. */

import { backend, type Chapter, type ExportFormat, type ExportResult, type Preset } from "./ipc";
import type { ReadingDirection } from "./ipc.domain";

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
  /** Who the book credits; empty when the source names nobody. */
  authors: string[];
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
  /** JPEG quality of the pages; null keeps the profile's. */
  quality?: number | null;
  /** Write a bigger book as numbered parts of at most this many MB. */
  splitMb?: number | null;
  /** Which way the pages turn; null: the series preset's, else the source's. */
  direction?: ReadingDirection | null;
  /** The books a stopped run already wrote: Try again leaves them out and never writes over them. */
  kept?: KeptBook[];
};

/** Mirrors `volume::KeptBook`: a kept book's chapter ids and the file it was written to. */
export type KeptBook = { chapters: string[]; file: string };

/** What `estimate_export` is asked: each book's chapter folders with the dialog's choices. */
export type EstimateRequest = {
  books: string[][];
  device: string;
  preset: Preset;
  quality: number | null;
  /** Emailed: held to 50 MB, else 200 MB. */
  email: boolean;
  splitMb: number | null;
};

/** Mirrors `mangas_core::export::kindle::BookBudget`: a book's estimate against its budget. */
export type BookBudget = {
  pages: number;
  estimated_bytes: number;
  budget_bytes: number;
  over_budget: boolean;
  jpeg_quality: number;
  /** Files it is written as: its parts when split, else 1. */
  files: number;
  levers?: Levers | null;
};

/** Mirrors `kindle::Levers`: what brings an over-budget book under its budget. */
export type Levers = {
  split_bytes: number;
  chapters_per_book: number | null;
  jpeg_quality: number | null;
};

export type VolumesBackend = {
  /** The books an export would write, in order, from the downloaded chapters among the ids. */
  planVolumes(request: VolumeRequest): Promise<VolumePlan[]>;
  /**
   * Writes every planned book; resolves to one result per book. A book that fails stops the run:
   * rejects with `ExportStopped` (the books written before it, it, the ones not started).
   */
  exportVolumes(request: VolumeExport): Promise<ExportResult[]>;
  /** The names `names` would be written under in `dir` now: taken ones get ` (2)`, ` (3)`… */
  freeExportNames(dir: string, names: string[]): Promise<string[]>;
  /** Rejects with `couldn't list <dir>: …` when a split or replacing export into `dir` would. */
  checkExportDir(dir: string, split: boolean, replace: boolean): Promise<void>;
  /** The file name a book titled `title` is written under as `format` (shortened when too long). */
  exportFileName(title: string, format: ExportFormat): Promise<string>;
  /** Each book's size against its budget, from a few of its pages encoded for real. */
  estimateExport(request: EstimateRequest): Promise<BookBudget[]>;
  /** What makes each book fit, as its plan offers it (`kindle::book_levers`). */
  fitLevers(request: EstimateRequest): Promise<Levers[]>;
};

export const planVolumes = async (r: VolumeRequest) => (await backend()).planVolumes(r);
export const exportVolumes = async (r: VolumeExport) => (await backend()).exportVolumes(r);
export const freeExportNames = async (dir: string, names: string[]) =>
  (await backend()).freeExportNames(dir, names);
export const checkExportDir = async (dir: string, split: boolean, replace: boolean) =>
  (await backend()).checkExportDir(dir, split, replace);
export const exportFileName = async (title: string, format: ExportFormat = "epub") =>
  (await backend()).exportFileName(title, format);
export const estimateExport = async (r: EstimateRequest) => (await backend()).estimateExport(r);
export const fitLevers = async (r: EstimateRequest) => (await backend()).fitLevers(r);
