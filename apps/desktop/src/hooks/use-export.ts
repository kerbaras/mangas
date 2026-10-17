import { useCallback, useEffect, useRef, useState } from "react";
import { errorMessage } from "@/lib/format";
import { type ExportProgress, type ExportRequest, exportKindle, onExportProgress } from "@/lib/ipc";
import { notifyError } from "@/lib/notify";

export type ExportState =
  | { kind: "idle" }
  | { kind: "running"; progress: ExportProgress | null }
  | { kind: "done"; output: string }
  | { kind: "error"; message: string };

export function useExport() {
  const [state, setState] = useState<ExportState>({ kind: "idle" });
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

  const run = useCallback(async (request: ExportRequest) => {
    running.current = true;
    setState({ kind: "running", progress: null });
    try {
      const output = await exportKindle(request);
      setState({ kind: "done", output });
    } catch (e) {
      setState({ kind: "error", message: errorMessage(e) });
      notifyError("Export failed", e, { id: "export" });
    } finally {
      running.current = false;
    }
  }, []);

  const reset = useCallback(() => setState({ kind: "idle" }), []);
  return { state, run, reset };
}
