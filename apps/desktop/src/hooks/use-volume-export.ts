import { useCallback, useEffect, useRef, useState } from "react";
import { notifyExported } from "@/lib/export-summary";
import { errorMessage } from "@/lib/format";
import { type ExportProgress, type ExportResult, onExportProgress } from "@/lib/ipc";
import { exportVolumes, type VolumeExport } from "@/lib/ipc.volumes";
import { notifyError } from "@/lib/notify";

export type VolumeExportState =
  | { kind: "idle" }
  | { kind: "running"; progress: ExportProgress | null }
  | { kind: "done"; results: ExportResult[] }
  | { kind: "error"; message: string };

/** Sums the books of a volume export into the one-line toast single exports show. */
const combined = (results: ExportResult[]): ExportResult => ({
  output: results.length === 1 ? results[0].output : `${results.length} books`,
  pages: results.reduce((n, r) => n + r.pages, 0),
  skipped: results.reduce((n, r) => n + r.skipped, 0),
});

export function useVolumeExport() {
  const [state, setState] = useState<VolumeExportState>({ kind: "idle" });
  const running = useRef(false);

  useEffect(() => {
    let unlisten: (() => void) | null = null;
    let alive = true;
    onExportProgress((progress) => {
      if (running.current) setState({ kind: "running", progress });
    }).then((u) => {
      if (alive) unlisten = u;
      else u();
    });
    return () => {
      alive = false;
      unlisten?.();
    };
  }, []);

  /** Resolves to whether every book was written. */
  const run = useCallback(async (request: VolumeExport): Promise<boolean> => {
    running.current = true;
    setState({ kind: "running", progress: null });
    try {
      const results = await exportVolumes(request);
      setState({ kind: "done", results });
      notifyExported(combined(results));
      return true;
    } catch (e) {
      setState({ kind: "error", message: errorMessage(e) });
      notifyError("Export failed", e, { id: "export" });
      return false;
    } finally {
      running.current = false;
    }
  }, []);

  const reset = useCallback(() => setState({ kind: "idle" }), []);
  return { state, run, reset };
}
