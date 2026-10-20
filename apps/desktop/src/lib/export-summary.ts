/** What the user is told when an export finishes, including files it had to leave out. */
import { toast } from "sonner";
import type { ExportResult } from "./ipc";
import { openLogs } from "./ipc.diagnostics";
import { notifyError } from "./notify";
import { titleOf } from "./send-labels";

const fileName = (path: string): string =>
  path.slice(Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\")) + 1);

const count = (n: number, word: string): string => `${n} ${word}${n === 1 ? "" : "s"}`;

/** Toast title, plus a warning when files were skipped or the detectors couldn't run. */
export function exportSummary({ output, parts, pages, skipped, detectors_error }: ExportResult) {
  const warnings = [
    skipped > 0 && `${count(skipped, "unsupported file")} skipped — see logs`,
    detectors_error && `Cut without detectors: ${detectors_error}`,
  ].filter(Boolean);
  return {
    title:
      parts.length > 1
        ? `Exported ${count(pages, "page")} in ${parts.length} parts, from ${fileName(parts[0])}`
        : `Exported ${count(pages, "page")} to ${fileName(output)}`,
    warning: warnings.length ? warnings.join(". ") : null,
  };
}

/** Every file a result wrote: its parts when the book was split, else the one book. */
export const filesOf = (r: ExportResult): string[] => (r.parts.length > 0 ? r.parts : [r.output]);

/** Every file a result wrote and the title its delivery sends it under: a part's is
 * "Title (part 2 of 3)", like the book's own (`export::kindle::parts::part_title`). */
export function booksOf(r: ExportResult): { file: string; title: string }[] {
  if (r.parts.length === 0) return [{ file: r.output, title: titleOf(r.output) }];
  const base = titleOf(r.parts[0]).replace(/ \(part 1\)$/, "");
  const total = r.parts.length;
  return r.parts.map((file, i) => ({ file, title: `${base} (part ${i + 1} of ${total})` }));
}

/** Several books reported as one: pages and skipped files add up. */
export const combineResults = (results: ExportResult[]): ExportResult => ({
  output: results.length === 1 ? results[0].output : `${results.length} books`,
  parts: results.length === 1 ? results[0].parts : [],
  pages: results.reduce((n, r) => n + r.pages, 0),
  skipped: results.reduce((n, r) => n + r.skipped, 0),
  detectors_error: results.find((r) => r.detectors_error)?.detectors_error ?? null,
  oversized: results.flatMap((r) => r.oversized ?? []),
});

const openLogsOrSay = () =>
  openLogs().catch((e: unknown) => notifyError("Couldn't open the logs folder", e));

export function notifyExported(result: ExportResult) {
  const { title, warning } = exportSummary(result);
  if (!warning) {
    toast.success(title, { id: "export", description: null, action: null });
    return;
  }
  toast.warning(title, {
    id: "export",
    description: warning,
    duration: 15_000,
    action: { label: "Open logs", onClick: () => void openLogsOrSay() },
  });
}
