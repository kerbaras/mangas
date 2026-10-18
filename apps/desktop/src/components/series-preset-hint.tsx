import type { ExportPreset } from "@/lib/ipc.cut-preview";

/** Says the export applies the series' saved preset (`null`: no preset, nothing shown). */
export function SeriesPresetHint({ preset }: { preset: ExportPreset | null }) {
  if (!preset) return null;
  return <p className="text-caption text-ink-3">Using series preset</p>;
}
