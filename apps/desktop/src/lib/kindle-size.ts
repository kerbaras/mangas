/** The Send-to-Kindle size budget the export dialog shows (`export::kindle::budget`). */
import type { ExportResult } from "./ipc.export";
import type { BookBudget, Levers, VolumePlan } from "./ipc.volumes";

/** Amazon's Send-to-Kindle email limit for one attached file, in MB (10^6 bytes). */
export const SEND_TO_KINDLE_MB = 50;
/** The part size offered: under the limit with room for what the estimate misses. */
export const PART_MB = 45;
/** Amazon's Send-to-Kindle web upload limit for one file, in MB: the budget of every other delivery. */
export const WEB_MB = 200;
/** The part size offered for the web limit. */
export const WEB_PART_MB = 180;
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

/** The downloaded chapter folders of each planned book, empty for a book with none. */
export const bookDirs = (plans: VolumePlan[]): string[][] =>
  plans.map((p) => p.chapters.flatMap((c) => c.local_path ?? []));

/** `budgets` of the books with folders among `dirs`, back in plan order (null: nothing to size). */
export function alignBudgets(dirs: string[][], budgets: BookBudget[]): (BookBudget | null)[] {
  let next = 0;
  return dirs.map((d) => (d.length > 0 ? (budgets[next++] ?? null) : null));
}

/** "212 MB of 200 MB": a book's estimate against its budget, as a preview row shows it. */
export const againstBudget = (b: BookBudget) =>
  `${megabytes(b.estimated_bytes)} of ${megabytes(b.budget_bytes)}`;

/** The levers that fit every book of `levers`: the fewest chapters and the lowest quality. */
export function combinedLevers(levers: Levers[]): Levers | null {
  if (levers.length === 0) return null;
  const lowest = (pick: (l: Levers) => number | null) => {
    const all = levers.map(pick);
    return all.some((v) => v === null) ? null : Math.min(...(all as number[]));
  };
  return {
    split_bytes: levers[0].split_bytes,
    chapters_per_book: lowest((l) => l.chapters_per_book),
    jpeg_quality: lowest((l) => l.jpeg_quality),
  };
}

/** What the size line says about `budgets` (core's numbers and over-budget flags, as is). */
export function budgetNote(budgets: BookBudget[]) {
  const largest = budgets.reduce((a, b) => (b.estimated_bytes > a.estimated_bytes ? b : a));
  const parts = largest.files > 1 ? ` in ${largest.files} parts` : "";
  const size = `about ${megabytes(largest.estimated_bytes)}${parts} of ${megabytes(largest.budget_bytes)}`;
  const label = `${budgets.length > 1 ? "Largest book: " : ""}${size}, ${largest.pages} pages, JPEG quality ${largest.jpeg_quality}`;
  const over = budgets.filter((b) => b.over_budget);
  return {
    label: label[0].toUpperCase() + label.slice(1),
    over: over.length,
    budgetMb: largest.budget_bytes / 1_000_000,
    levers: combinedLevers(over.flatMap((b) => b.levers ?? [])),
  };
}

/** The written files over the budget of the delivery (`email` or not), as measured on disk. */
export const oversizedOf = (results: ExportResult[], email: boolean) =>
  results
    .flatMap((r) => r.oversized ?? [])
    .filter((o) => o.bytes > (email ? SEND_TO_KINDLE_MB : WEB_MB) * 1_000_000);

/** The chapter folders of each written book with a file over the budget: its plan by chapter ids,
 * else by position (`results` are in plan order). */
export const oversizedDirs = (results: ExportResult[], plans: VolumePlan[], email: boolean) =>
  results.flatMap((r, i) => {
    if (oversizedOf([r], email).length === 0) return [];
    const ids = r.chapters;
    const plan = ids ? plans.find((p) => p.chapters.every((c) => ids.includes(c.id))) : plans[i];
    const dirs = plan ? bookDirs([plan])[0] : [];
    return dirs.length > 0 ? [dirs] : [];
  });
