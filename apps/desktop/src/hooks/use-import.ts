import { useCallback, useState } from "react";
import { type ImportKind, pickImportPaths } from "@/lib/dialogs";
import { errorMessage } from "@/lib/format";
import { importSummary } from "@/lib/import-summary";
import { type ImportReport, importLocal } from "@/lib/ipc";

export type ImportStatus =
  | { kind: "idle" }
  | { kind: "busy" }
  | { kind: "done"; message: string }
  | { kind: "error"; message: string };

/** Picks paths with the native dialog, imports them and keeps a one-line status. */
export function useImport(onImported: (report: ImportReport) => void) {
  const [status, setStatus] = useState<ImportStatus>({ kind: "idle" });

  const start = useCallback(
    async (kind: ImportKind) => {
      const paths = await pickImportPaths(kind);
      if (paths.length === 0) return;
      setStatus({ kind: "busy" });
      try {
        const report = await importLocal(paths);
        setStatus({ kind: "done", message: importSummary(report) });
        onImported(report);
      } catch (e) {
        setStatus({ kind: "error", message: errorMessage(e) });
      }
    },
    [onImported],
  );

  const dismiss = useCallback(() => setStatus({ kind: "idle" }), []);
  return { status, start, dismiss };
}
