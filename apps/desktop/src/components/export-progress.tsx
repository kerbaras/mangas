import { FolderOpen } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Progress } from "@/components/ui/progress";
import type { ExportState } from "@/hooks/use-export";
import { revealFolder } from "@/lib/dialogs";
import { parentDir } from "@/lib/export-name";
import { combineResults, exportSummary } from "@/lib/export-summary";

export function ExportProgressPanel({
  state,
  onReset,
}: {
  state: ExportState;
  onReset: () => void;
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
    return (
      <div
        className="rounded-md border border-accent/40 bg-accent-soft/40 p-3 text-body"
        role="alert"
      >
        <p className="font-medium text-accent">Export failed</p>
        <p className="mt-1 text-ink-2">{state.message}</p>
        <Button size="sm" className="mt-3" onClick={onReset}>
          Try again
        </Button>
      </div>
    );
  }
  const { results } = state;
  const [first] = results;
  const { pages } = combineResults(results);
  const { warning } = exportSummary(combineResults(results));
  const folder = first ? parentDir(first.output) : "";
  return (
    <div
      className="flex items-center justify-between gap-3 rounded-md border border-stroke bg-surface p-3"
      role="status"
    >
      <div className="min-w-0">
        <p className="text-body font-medium">
          {results.length === 1
            ? `Exported ${pages} ${pages === 1 ? "page" : "pages"}`
            : `Wrote ${results.length} books`}
        </p>
        <p className="truncate text-caption text-ink-3" title={first?.output}>
          {results.length === 1 ? first.output : folder}
        </p>
        {warning && <p className="text-caption text-accent">{warning}</p>}
      </div>
      <Button size="sm" onClick={() => revealFolder(folder)}>
        <FolderOpen /> Open folder
      </Button>
    </div>
  );
}
