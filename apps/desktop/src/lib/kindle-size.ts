/** The Send-to-Kindle size rule the export dialog warns about (`export::kindle::parts`). */
import type { Estimate, VolumePlan } from "./ipc.volumes";

/** Amazon's Send-to-Kindle email limit for one attached file, in MB (10^6 bytes). */
export const SEND_TO_KINDLE_MB = 50;
/** The part size offered: under the limit with room for what the estimate misses. */
export const PART_MB = 45;
/** JPEG qualities the core accepts (`export::page::JPEG_QUALITY`). */
export const JPEG_QUALITIES = [80, 70, 60, 50] as const;

/** `stored` qualities with `device`'s set to `quality`, or removed for the device's own. */
export const withQuality = (
  stored: Record<string, number>,
  device: string,
  quality: number | null,
): Record<string, number> => {
  const rest = Object.fromEntries(Object.entries(stored).filter(([id]) => id !== device));
  return quality === null ? rest : { ...rest, [device]: quality };
};

export const megabytes = (bytes: number): string => {
  const mb = bytes / 1_000_000;
  return `${mb < 10 ? mb.toFixed(1) : Math.round(mb)} MB`;
};

/** The downloaded chapter folders of each planned book; the estimate measures the largest. */
export function downloadedBooks(plans: VolumePlan[]): string[][] {
  return plans
    .map((p) => p.chapters.flatMap((c) => c.local_path ?? []))
    .filter((dirs) => dirs.length > 0);
}

/** How far off the estimate may be either way (`export::kindle::estimate` samples pages). */
const ESTIMATE_SLACK = 0.15;

/** The part count the estimate allows: ", about 3 parts", ", 2 or 3 parts", or "" for one. */
function partsNote(bytes: number, part: number): string {
  const count = (b: number) => Math.max(1, Math.ceil(b / part));
  const [lo, hi] = [count(bytes * (1 - ESTIMATE_SLACK)), count(bytes * (1 + ESTIMATE_SLACK))];
  if (hi === 1) return "";
  if (lo === hi) return `, about ${lo} parts`;
  return `, ${lo} ${hi - lo === 1 ? "or" : "to"} ${hi} parts`;
}

/** What the size line says, and whether a file written would be over the email limit. */
export function sizeNote(e: Estimate, books: number, splitMb: number | null) {
  const size = `about ${megabytes(e.bytes)}, ${e.pages} pages`;
  const limit = SEND_TO_KINDLE_MB * 1_000_000;
  const part = splitMb ? splitMb * 1_000_000 : Number.POSITIVE_INFINITY;
  const label = `${books > 1 ? "Largest book: " : ""}${size}${partsNote(e.bytes, part)}`;
  return { label: label[0].toUpperCase() + label.slice(1), over: Math.min(e.bytes, part) > limit };
}
