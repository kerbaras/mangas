import { FolderOpen } from "lucide-react";
import { Button } from "@/components/ui/button";
import { revealFolder } from "@/lib/dialogs";
import { describeError } from "@/lib/errors";
import { parentDir } from "@/lib/export-name";
import type { ExportStopped } from "@/lib/export-stopped";
import type { ExportResult } from "@/lib/ipc";
import { isFinalError } from "@/lib/ipc.downloads";

const fileName = (path: string): string =>
  path.slice(Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\")) + 1);

const books = (n: number) => `${n} ${n === 1 ? "book" : "books"}`;

/** A volume export that stopped at a failed book: what failed, what was kept, what wasn't written. */
export function ExportStoppedPanel({
  stopped,
  kept,
  onReset,
  canRetry = true,
}: {
  stopped: ExportStopped;
  /** Books an earlier stopped run wrote, which this retry left as they were. */
  kept?: ExportResult[];
  onReset: () => void;
  /** False while the form can't export (names or settings still loading). */
  canRetry?: boolean;
}) {
  const { results, failed, notWritten } = stopped;
  const folder = parentDir(failed.output);
  const final = isFinalError(failed.reason);
  const { hint } = describeError(failed.reason);
  return (
    <div
      className="rounded-md border border-accent/40 bg-accent-soft/40 p-3 text-body"
      role="alert"
    >
      <p className="font-medium text-accent-text">Export stopped</p>
      <p className="mt-1 text-ink-2 [overflow-wrap:anywhere]">
        {fileName(failed.output)} failed and was not saved: {failed.reason}
      </p>
      <p className="mt-1 text-ink-2">
        {results.length
          ? `Kept the ${books(results.length)} written before it: ${results
              .map((r) => fileName(r.output))
              .join(", ")}.`
          : "No book was written."}
      </p>
      <KeptNote kept={kept} />
      {notWritten.length > 0 && (
        <p className="mt-1 text-ink-2 [overflow-wrap:anywhere]">
          Not written: {notWritten.map((b) => fileName(b.output)).join(", ")}.
        </p>
      )}
      {final && hint && <p className="mt-1 text-ink-2">{hint}</p>}
      <div className="mt-3 flex gap-2">
        {!final && (
          <Button size="sm" disabled={!canRetry} onClick={onReset}>
            Try again
          </Button>
        )}
        {results.length > 0 && (
          <Button size="sm" onClick={() => revealFolder(folder)}>
            <FolderOpen /> Open folder
          </Button>
        )}
      </div>
    </div>
  );
}

/** The books an earlier, stopped run wrote: "Try again" leaves them as they are. */
export function KeptNote({
  kept = [],
  className = "mt-1 text-ink-2",
}: {
  kept?: ExportResult[];
  className?: string;
}) {
  if (kept.length === 0) return null;
  return (
    <p className={`${className} [overflow-wrap:anywhere]`}>
      Already there from the last try: {kept.map((r) => fileName(r.output)).join(", ")}.
    </p>
  );
}
