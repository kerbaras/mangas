/** What the user is told when an export finishes, including files it had to leave out. */
import { toast } from "sonner";
import type { ExportResult } from "./ipc";
import { openLogs } from "./ipc.diagnostics";
import { notifyError } from "./notify";

const fileName = (path: string): string =>
  path.slice(Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\")) + 1);

const count = (n: number, word: string): string => `${n} ${word}${n === 1 ? "" : "s"}`;

/** Toast title, plus a warning line when unsupported files were skipped. */
export function exportSummary({ output, pages, skipped }: ExportResult) {
  return {
    title: `Exported ${count(pages, "page")} to ${fileName(output)}`,
    warning: skipped > 0 ? `${count(skipped, "unsupported file")} skipped — see logs` : null,
  };
}

/** Several books reported as one: pages and skipped files add up. */
export const combineResults = (results: ExportResult[]): ExportResult => ({
  output: results.length === 1 ? results[0].output : `${results.length} books`,
  pages: results.reduce((n, r) => n + r.pages, 0),
  skipped: results.reduce((n, r) => n + r.skipped, 0),
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
