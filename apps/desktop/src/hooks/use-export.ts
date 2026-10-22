import { useCallback, useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import { parentDir } from "@/lib/export-name";
import { ExportStopped } from "@/lib/export-stopped";
import { combineResults, notifyExported } from "@/lib/export-summary";
import { errorMessage } from "@/lib/format";
import type { Folders } from "@/lib/io-hint";
import { type ExportProgress, type ExportResult, onExportProgress } from "@/lib/ipc";
import { notifyError } from "@/lib/notify";
import { throttleProgress } from "@/lib/progress-throttle";

export type ExportState =
  | { kind: "idle" }
  | { kind: "running"; progress: ExportProgress | null }
  | { kind: "done"; results: ExportResult[]; kept?: ExportResult[] }
  | { kind: "error"; message: string; folder?: string; before?: Retry }
  | { kind: "stopped"; stopped: ExportStopped; before: Retry };

/**
 * The books earlier stopped runs wrote: "Try again" leaves them out (matched by chapter ids, since
 * downloads finishing meanwhile can change the plan) and reports them as `kept`. A plain failure
 * in between writes nothing, so it keeps them for the next try.
 */
export type Retry = { kept: ExportResult[] };
const FRESH: Retry = { kept: [] };

export function retryOf(state: ExportState): Retry {
  if (state.kind === "error") return state.before ?? FRESH;
  if (state.kind !== "stopped") return FRESH;
  const { before, stopped } = state;
  return { kept: [...before.kept, ...stopped.results] };
}

type Options = {
  /** Whether the export dialog is open, so its panel can show the failure. */
  visible?: () => boolean;
  /** Where the books go, so an io error's hint can name it. */
  folder?: string;
};

/**
 * The dialog's panel is out of sight while the window is minimised or another modal (Smart-cut
 * options) is open over the export dialog; a toast reports the failure then.
 */
const panelInSight = () =>
  document.visibilityState !== "hidden" && document.querySelectorAll('[role="dialog"]').length <= 1;

/** Runs one export job (one or more books) and follows its progress events. */
export function useExport({ visible = () => false, folder }: Options = {}) {
  const [state, setState] = useState<ExportState>({ kind: "idle" });
  const shown = useRef(visible);
  shown.current = visible;
  /** One surface for a failure: the dialog's panel while it is open and seen, else a toast. */
  const report = useCallback((title: string, e: unknown, folders: Folders) => {
    if (shown.current() && panelInSight()) toast.dismiss("export");
    else notifyError(title, e, { id: "export", folders });
  }, []);
  const current = useRef(state);
  current.current = state;
  /** Runs that finished; one that stopped or failed keeps the books written for "Try again". */
  const [settled, setSettled] = useState(0);
  const folderRef = useRef(folder);
  folderRef.current = folder;
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
      const before = retryOf(current.current);
      const folder = folderRef.current;
      running.current = true;
      throttle.reset();
      setState({ kind: "running", progress: null });
      try {
        const results = await job();
        setState({ kind: "done", results, kept: before.kept });
        setSettled((n) => n + 1);
        notifyExported(combineResults(results));
        return results;
      } catch (e) {
        if (e instanceof ExportStopped) {
          setState({ kind: "stopped", stopped: e, before });
          report("Export stopped", e, { export: parentDir(e.failed.output) });
          return e.results.length ? e.results : null;
        }
        setState({ kind: "error", message: errorMessage(e), folder, before });
        report("Export failed", e, { export: folder });
        return null;
      } finally {
        running.current = false;
        throttle.reset();
      }
    },
    [throttle, report],
  );

  const reset = useCallback(() => setState({ kind: "idle" }), []);
  return { state, run, reset, settled, retry: retryOf(state) };
}
