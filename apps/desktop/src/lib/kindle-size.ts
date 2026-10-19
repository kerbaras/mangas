/** The Send-to-Kindle size rule the export dialog warns about (`export::kindle::parts`). */
import type { Estimate, VolumePlan } from "./ipc.volumes";

/** Amazon's Send-to-Kindle email limit for one attached file, in MB (10^6 bytes). */
export const SEND_TO_KINDLE_MB = 50;
/** The part size offered: under the limit with room for what the estimate misses. */
export const PART_MB = 45;
/** JPEG qualities the core accepts (`export::page::JPEG_QUALITY`). */
export const JPEG_QUALITIES = [80, 70, 60, 50] as const;

/** How the dialog keeps a book under the limit: smaller pages, numbered parts, or both. */
export type Shrink = { quality: number | null; splitMb: number | null };

export const NO_SHRINK: Shrink = { quality: null, splitMb: null };

export const megabytes = (bytes: number): string => {
  const mb = bytes / 1_000_000;
  return `${mb < 10 ? mb.toFixed(1) : Math.round(mb)} MB`;
};

/** The book the estimate is for: the one with the most downloaded chapters. */
export function biggestBook(plans: VolumePlan[]): string[] {
  const dirs = plans.map((p) => p.chapters.flatMap((c) => c.local_path ?? []));
  return dirs.reduce<string[]>((big, d) => (d.length > big.length ? d : big), []);
}

/** What the size line says, and whether a file written would be over the email limit. */
export function sizeNote(e: Estimate, books: number, splitMb: number | null) {
  const size = `about ${megabytes(e.bytes)}, ${e.pages} pages`;
  const limit = SEND_TO_KINDLE_MB * 1_000_000;
  const part = splitMb ? splitMb * 1_000_000 : Number.POSITIVE_INFINITY;
  const parts = e.bytes > part ? Math.ceil(e.bytes / part) : 1;
  const label = `${books > 1 ? "Largest book: " : ""}${size}${parts > 1 ? `, about ${parts} parts` : ""}`;
  return { label: label[0].toUpperCase() + label.slice(1), over: Math.min(e.bytes, part) > limit };
}
