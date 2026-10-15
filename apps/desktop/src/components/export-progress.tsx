import { FolderOpen } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Progress } from "@/components/ui/progress";
import type { ExportState } from "@/hooks/use-export";
import { revealFolder } from "@/lib/dialogs";

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
              {p.current}/{p.total}
            </span>
          )}
        </div>
        <Progress value={p?.current ?? 0} max={p?.total ?? 1} className="mt-2" />
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
  return (
    <div
      className="flex items-center justify-between rounded-md border border-stroke bg-surface p-3"
      role="status"
    >
      <div className="min-w-0">
        <p className="text-body font-medium">Exported</p>
        <p className="truncate text-caption text-ink-3" title={state.output}>
          {state.output}
        </p>
      </div>
      <Button size="sm" onClick={() => revealFolder(state.output)}>
        <FolderOpen /> Open folder
      </Button>
    </div>
  );
}
