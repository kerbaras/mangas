/** What the user is told when an export finishes, including files it had to leave out. */
import { toast } from "sonner";
import type { ExportResult } from "./ipc";
import { openLogs } from "./ipc.diagnostics";

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

export function notifyExported(result: ExportResult) {
  const { title, warning } = exportSummary(result);
  if (!warning) {
    toast.success(title, { id: "export" });
    return;
  }
  toast.warning(title, {
    id: "export",
    description: warning,
    duration: 15_000,
    action: { label: "Open logs", onClick: () => void openLogs() },
  });
}
