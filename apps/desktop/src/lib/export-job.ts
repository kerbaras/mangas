/** Turns the export dialog's choices into the backend call that writes the books. */
import { type ExportPick, exportTitle, joinPath } from "./export-name";
import { type BundleMode, groupingOf } from "./export-plan";
import { ExportStopped } from "./export-stopped";
import { type ExportFormat, type ExportResult, exportChapters, type Preset } from "./ipc";
import { exportFileName, exportVolumes } from "./ipc.volumes";
import { storageChanged } from "./storage-changed";

export type ExportChoices = {
  picks: ExportPick[];
  mode: BundleMode;
  size: number;
  title: string;
  device: string;
  format: ExportFormat;
  preset: Preset;
  detectors: boolean;
  /** Null for the default export folder, resolved by the backend. */
  outputDir: string | null;
  /** Write over files already in the folder instead of ` (2)` copies. */
  replace: boolean;
  /** JPEG quality of the pages; null keeps the profile's. */
  quality: number | null;
  /** Write a bigger book as numbered parts of at most this many MB. */
  splitMb: number | null;
};

/**
 * One series goes through the volume planner (every bundle mode, series cover, TOC, metadata);
 * chapters from several series are joined into one book by `export_chapters`.
 */
export function exportJob(c: ExportChoices): () => Promise<ExportResult[]> {
  const write = writeBooks(c);
  return async () => {
    try {
      return reloadDetectors(await write());
    } catch (e) {
      if (e instanceof ExportStopped) reloadDetectors(e.results);
      throw e;
    }
  };
}

/** The detector status now holds a failure: the "Use detectors" switch shows it. */
function reloadDetectors(results: ExportResult[]): ExportResult[] {
  if (results.some((r) => r.detectors_error)) storageChanged();
  return results;
}

function writeBooks(c: ExportChoices): () => Promise<ExportResult[]> {
  const { device, format, preset, detectors, replace, quality, splitMb } = c;
  const common = { device, format, preset, detectors, replace, quality, splitMb };
  if (c.picks.length === 1) {
    const [{ manga, chapters }] = c.picks;
    return () =>
      exportVolumes({
        source: manga.source,
        mangaId: manga.id,
        chapterIds: chapters.map((ch) => ch.id),
        grouping: groupingOf(c.mode, c.size),
        title: c.title,
        ...common,
        outputDir: c.outputDir,
      });
  }
  const title = c.title.trim() || exportTitle(c.picks);
  const dirs = c.picks.flatMap((p) => p.chapters.flatMap((ch) => ch.local_path ?? []));
  return async () => {
    const file = await exportFileName(title, c.format);
    const output = c.outputDir ? joinPath(c.outputDir, file) : file;
    return [await exportChapters({ chapter_dirs: dirs, ...common, output, title })];
  };
}
