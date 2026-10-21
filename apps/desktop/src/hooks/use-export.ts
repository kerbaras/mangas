import { useCallback, useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import { ExportStopped } from "@/lib/export-stopped";
import { combineResults, notifyExported } from "@/lib/export-summary";
import { errorMessage } from "@/lib/format";
import { type ExportProgress, type ExportResult, onExportProgress } from "@/lib/ipc";
import { notifyError } from "@/lib/notify";
import { throttleProgress } from "@/lib/progress-throttle";

export type ExportState =
  | { kind: "idle" }
  | { kind: "running"; progress: ExportProgress | null }
  | { kind: "done"; results: ExportResult[] }
  | { kind: "error"; message: string }
  | { kind: "stopped"; stopped: ExportStopped };

type Options = {
  /** Whether the export dialog is open, so its panel can show the failure. */
  visible?: () => boolean;
};

/**
 * The dialog's panel is out of sight while the window is minimised or another modal (Smart-cut
 * options) is open over the export dialog; a toast reports the failure then.
 */
const panelInSight = () =>
  document.visibilityState !== "hidden" && document.querySelectorAll('[role="dialog"]').length <= 1;

/** Runs one export job (one or more books) and follows its progress events. */
export function useExport({ visible = () => false }: Options = {}) {
  const [state, setState] = useState<ExportState>({ kind: "idle" });
  const shown = useRef(visible);
  shown.current = visible;
  /** One surface for a failure: the dialog's panel while it is open and seen, else a toast. */
  const report = useCallback((title: string, e: unknown) => {
    if (shown.current() && panelInSight()) toast.dismiss("export");
    else notifyError(title, e, { id: "export" });
  }, []);
  const running = useRef(false);
  const [throttle] = useState(() =>
    throttleProgress((progress) => {
      if (!running.current) return;
      if (progress.notice) toast.info(progress.notice, { id: "export-notice" });
      setState({ kind: "running", progress });
    }),
  );

  useEffect(() => {
    let unlisten: (() => void) | null = null;
    let alive = true;
    onExportProgress((progress) => {
      if (running.current) throttle.push(progress);
    }).then((u) => {
      if (alive) unlisten = u;
      else u();
    });
    return () => {
      alive = false;
      unlisten?.();
      throttle.reset();
    };
  }, [throttle]);

  /**
   * Resolves to the written books, or null when the export failed. A run that stopped part-way
   * resolves to the complete books written before the failure (null when there are none).
   */
  const run = useCallback(
    async (job: () => Promise<ExportResult[]>) => {
      running.current = true;
      throttle.reset();
      setState({ kind: "running", progress: null });
      try {
        const results = await job();
        setState({ kind: "done", results });
        notifyExported(combineResults(results));
        return results;
      } catch (e) {
        if (e instanceof ExportStopped) {
          setState({ kind: "stopped", stopped: e });
          report("Export stopped", e);
          return e.results.length ? e.results : null;
        }
        setState({ kind: "error", message: errorMessage(e) });
        report("Export failed", e);
        return null;
      } finally {
        running.current = false;
        throttle.reset();
      }
    },
    [throttle, report],
  );

  const reset = useCallback(() => setState({ kind: "idle" }), []);
  return { state, run, reset };
}
