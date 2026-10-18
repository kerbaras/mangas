/** The toast an import ends with: the summary, plus why entries or pages were left out. */
import { toast } from "sonner";
import { importSummary } from "./import-summary";
import type { ImportReport } from "./ipc";

type Skipped = ImportReport["skipped"];

/** Collapsed by default so a long list doesn't cover the screen; scrolls past a few lines. */
export function SkippedList({ skipped }: { skipped: Skipped }) {
  const label =
    skipped.length === 1 ? "Why 1 item was skipped" : `Why ${skipped.length} items were skipped`;
  return (
    <details className="group">
      <summary className="cursor-pointer select-none text-ink-2 hover:text-ink">{label}</summary>
      <ul className="mt-1 flex max-h-40 flex-col gap-0.5 overflow-y-auto pr-1">
        {skipped.map((s) => (
          <li key={`${s.name}\u0000${s.reason}`} className="break-all">
            <span className="text-ink">{s.name}</span>
            <span className="text-ink-3"> — {s.reason}</span>
          </li>
        ))}
      </ul>
    </details>
  );
}

export function notifyImported(report: ImportReport, id: string) {
  const title = importSummary(report);
  if (report.skipped.length === 0) {
    toast.success(title, { id, description: null });
    return;
  }
  toast.success(title, {
    id,
    description: <SkippedList skipped={report.skipped} />,
    duration: 15_000,
  });
}
