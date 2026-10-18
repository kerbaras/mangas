/** The toast an import ends with: the summary, plus why entries or pages were left out. */
import { toast } from "sonner";
import { importSummary } from "./import-summary";
import type { ImportReport } from "./ipc";

type Skipped = ImportReport["skipped"];

/**
 * Always expanded: sonner measures a toast's height only when it mounts or its content changes,
 * so a toggle would spill out of the card. The list scrolls past a few lines instead.
 */
export function SkippedList({ skipped }: { skipped: Skipped }) {
  const label =
    skipped.length === 1 ? "1 item was skipped:" : `${skipped.length} items were skipped:`;
  return (
    <div className="flex flex-col gap-1">
      <p className="text-ink-2">{label}</p>
      <ul
        aria-label="Skipped items"
        className="flex max-h-28 flex-col gap-0.5 overflow-y-auto overscroll-contain pr-1"
      >
        {skipped.map((s) => (
          <li key={`${s.name}\u0000${s.reason}`} className="break-words">
            <span className="text-ink">{s.name}</span>
            <span className="text-ink-3"> — {s.reason}</span>
          </li>
        ))}
      </ul>
    </div>
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
