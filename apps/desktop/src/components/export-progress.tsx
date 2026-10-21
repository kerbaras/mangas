import { FolderOpen } from "lucide-react";
import { ExportStoppedPanel, KeptNote } from "@/components/export-stopped-panel";
import { Hint } from "@/components/hint";
import { Button } from "@/components/ui/button";
import { Progress } from "@/components/ui/progress";
import type { ExportState } from "@/hooks/use-export";
import { revealFolder } from "@/lib/dialogs";
import { describeError } from "@/lib/errors";
import { parentDir } from "@/lib/export-name";
import { combineResults, exportSummary } from "@/lib/export-summary";
import { isFinalError } from "@/lib/ipc.downloads";

/** Export progress, its failure with "Try again" (`onRetry` exports again), or what it wrote. */
export function ExportProgressPanel({
  state,
  onRetry,
  canRetry = true,
}: {
  state: ExportState;
  onRetry: () => void;
  /** False while the form can't export (names or settings still loading). */
  canRetry?: boolean;
}) {
  if (state.kind === "idle") return null;
  if (state.kind === "running") {
    const p = state.progress;
    return (
      <div className="rounded-md border border-stroke bg-surface p-3" aria-live="polite">
        <div className="flex items-center justify-between text-body">
          <span>{p?.stage ?? "Starting…"}</span>
          {p && (
            <span className="text-caption text-ink-3 tabular-nums">
              {p.total === null ? p.current : `${p.current}/${p.total}`}
            </span>
          )}
        </div>
        <Progress value={p?.total ? p.current : 0} max={p?.total ?? 1} className="mt-2" />
      </div>
    );
  }
  if (state.kind === "error") {
    const final = isFinalError(state.message);
    const { hint } = describeError(state.message);
    return (
      <div
        className="rounded-md border border-accent/40 bg-accent-soft/40 p-3 text-body"
        role="alert"
      >
        <p className="font-medium text-accent-text">Export failed</p>
        <p className="mt-1 text-ink-2 [overflow-wrap:anywhere]">{state.message}</p>
        {final && hint && <p className="mt-1 text-ink-2">{hint}</p>}
        {!final && (
          <Button size="sm" className="mt-3" disabled={!canRetry} onClick={onRetry}>
            Try again
          </Button>
        )}
      </div>
    );
  }
  if (state.kind === "stopped")
    return (
      <ExportStoppedPanel
        stopped={state.stopped}
        kept={state.before.kept}
        onReset={onRetry}
        canRetry={canRetry}
      />
    );
  const { results } = state;
  const [first] = results;
  const { pages } = combineResults(results);
  const { warning } = exportSummary(combineResults(results));
  const folder = first ? parentDir(first.output) : "";
  const parts = results.length === 1 ? first.parts.length : 0;
  return (
    <div
      className="flex items-center justify-between gap-3 rounded-md border border-stroke bg-surface p-3"
      role="status"
    >
      <div className="min-w-0">
        <p className="text-body font-medium">
          {results.length === 1
            ? `Exported ${pages} ${pages === 1 ? "page" : "pages"}${parts > 1 ? ` in ${parts} parts` : ""}`
            : `Wrote ${results.length} books`}
        </p>
        <Hint hint={first?.output}>
          <p className="truncate text-caption text-ink-3">
            {results.length === 1 && parts < 2 ? first.output : folder}
          </p>
        </Hint>
        <KeptNote kept={state.kept} className="text-caption text-ink-3" />
        {warning && (
          <p className="text-caption text-accent-text [overflow-wrap:anywhere]">{warning}</p>
        )}
      </div>
      <Button size="sm" onClick={() => revealFolder(folder)}>
        <FolderOpen /> Open folder
      </Button>
    </div>
  );
}
