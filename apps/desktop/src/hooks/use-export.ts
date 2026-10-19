import { useCallback, useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import { ExportStopped } from "@/lib/export-stopped";
import { combineResults, notifyExported } from "@/lib/export-summary";
import { errorMessage } from "@/lib/format";
import { type ExportProgress, type ExportResult, onExportProgress } from "@/lib/ipc";
import { notifyError } from "@/lib/notify";

export type ExportState =
  | { kind: "idle" }
  | { kind: "running"; progress: ExportProgress | null }
  | { kind: "done"; results: ExportResult[] }
  | { kind: "error"; message: string }
  | { kind: "stopped"; stopped: ExportStopped };

/** Runs one export job (one or more books) and follows its progress events. */
export function useExport() {
  const [state, setState] = useState<ExportState>({ kind: "idle" });
  const running = useRef(false);

  useEffect(() => {
    let unlisten: (() => void) | null = null;
    let alive = true;
    onExportProgress((progress) => {
      if (!running.current) return;
      if (progress.notice) toast.info(progress.notice, { id: "export-notice" });
      setState({ kind: "running", progress });
    }).then((u) => {
      if (alive) unlisten = u;
      else u();
    });
    return () => {
      alive = false;
      unlisten?.();
    };
  }, []);

  /**
   * Resolves to the written books, or null when the export failed. A run that stopped part-way
   * resolves to the complete books written before the failure (null when there are none).
   */
  const run = useCallback(async (job: () => Promise<ExportResult[]>) => {
    running.current = true;
    setState({ kind: "running", progress: null });
    try {
      const results = await job();
      setState({ kind: "done", results });
      notifyExported(combineResults(results));
      return results;
    } catch (e) {
      if (e instanceof ExportStopped) {
        setState({ kind: "stopped", stopped: e });
        notifyError("Export stopped", e, { id: "export" });
        return e.results.length ? e.results : null;
      }
      setState({ kind: "error", message: errorMessage(e) });
      notifyError("Export failed", e, { id: "export" });
      return null;
    } finally {
      running.current = false;
    }
  }, []);

  const reset = useCallback(() => setState({ kind: "idle" }), []);
  return { state, run, reset };
}
