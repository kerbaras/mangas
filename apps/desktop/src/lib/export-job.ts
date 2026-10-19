/** Turns the export dialog's choices into the backend call that writes the books. */
import { type ExportPick, exportFileName, exportTitle, joinPath } from "./export-name";
import { type BundleMode, groupingOf } from "./export-plan";
import { type ExportFormat, type ExportResult, exportChapters, type Preset } from "./ipc";
import { exportVolumes } from "./ipc.volumes";
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
};

/**
 * One series goes through the volume planner (every bundle mode, series cover, TOC, metadata);
 * chapters from several series are joined into one book by `export_chapters`.
 */
export function exportJob(c: ExportChoices): () => Promise<ExportResult[]> {
  const write = writeBooks(c);
  return async () => {
    const results = await write();
    // The detector status now holds the failure: the "Use detectors" switch shows it.
    if (results.some((r) => r.detectors_error)) storageChanged();
    return results;
  };
}

function writeBooks(c: ExportChoices): () => Promise<ExportResult[]> {
  const common = { device: c.device, format: c.format, preset: c.preset, detectors: c.detectors };
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
  return async () => [
    await exportChapters({
      chapter_dirs: dirs,
      ...common,
      output: c.outputDir
        ? joinPath(c.outputDir, exportFileName(title, c.format))
        : exportFileName(title, c.format),
      title,
    }),
  ];
}
