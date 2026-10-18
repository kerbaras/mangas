/** Pure helpers for Settings → Storage and Backup. */
import type { Restored, StorageUsage } from "./ipc.storage";

/** "0 B", "999 B", "1.50 KB", "12.3 MB", "204 GB": decimal units, three significant digits. */
export function formatBytes(bytes: number): string {
  const units = ["B", "KB", "MB", "GB", "TB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1000 && unit < units.length - 1) {
    value /= 1000;
    unit += 1;
  }
  if (unit === 0) return `${Math.round(bytes)} B`;
  const digits = value >= 100 ? 0 : value >= 10 ? 1 : 2;
  return `${value.toFixed(digits)} ${units[unit]}`;
}

export type Segment = { key: string; label: string; bytes: number; color: string };

/** The usage bar's parts, in a fixed order; their colours match the legend. */
export const usageSegments = (u: StorageUsage): Segment[] => [
  { key: "chapters", label: "Chapters", bytes: u.chapters, color: "bg-accent" },
  { key: "models", label: "Detector models", bytes: u.models, color: "bg-warn" },
  { key: "covers", label: "Cover cache", bytes: u.covers, color: "bg-ok" },
  { key: "library", label: "Library and logs", bytes: u.library + u.logs, color: "bg-ink-3" },
];

/** Percentage of `total`, 0 when there's nothing to share. */
export const share = (bytes: number, total: number) => (total > 0 ? (bytes / total) * 100 : 0);

const plural = (n: number, one: string, many = `${one}s`) => `${n} ${n === 1 ? one : many}`;

/** One sentence per outcome of a backup import, for the toast. */
export function restoreSummary(r: Restored): string {
  const parts = [`Followed ${plural(r.added, "new title")}.`];
  if (r.already) parts.push(`${plural(r.already, "title")} already in your library.`);
  if (r.skipped.length) {
    parts.push(`Skipped ${r.skipped.map((s) => s.title).join(", ")}: provider not supported.`);
  }
  if (r.settings) parts.push("Preferences restored.");
  return parts.join(" ");
}

export const chaptersLabel = (n: number) => plural(n, "chapter");
