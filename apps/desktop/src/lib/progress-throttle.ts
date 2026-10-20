import type { ExportProgress } from "./ipc";

/** Export progress reaches the UI about 4 times a second. */
export const PROGRESS_MS = 250;

/**
 * Folds a stream of export progress events into about one update per `ms`. A new stage, a
 * notice and the last step of a stage go through at once; the latest step in between is
 * delivered when its window ends. Each update re-renders the export panel, and one per page
 * kept the webview at ~340 % CPU without a GPU.
 */
export function throttleProgress(
  deliver: (p: ExportProgress) => void,
  ms = PROGRESS_MS,
  now: () => number = Date.now,
) {
  let last: ExportProgress | null = null;
  let at = Number.NEGATIVE_INFINITY;
  let pending: ExportProgress | null = null;
  let timer: ReturnType<typeof setTimeout> | null = null;

  const stop = () => {
    if (timer) clearTimeout(timer);
    timer = null;
    pending = null;
  };
  const send = (p: ExportProgress) => {
    stop();
    last = p;
    at = now();
    deliver(p);
  };
  const urgent = (p: ExportProgress) =>
    !last ||
    p.stage !== last.stage ||
    p.notice !== undefined ||
    (p.total !== null && p.current >= p.total);

  return {
    push(p: ExportProgress) {
      if (urgent(p) || now() - at >= ms) return send(p);
      pending = p;
      timer ??= setTimeout(() => pending && send(pending), ms - (now() - at));
    },
    /** Drops a pending step and forgets the stage (the job ended, or a new one starts). */
    reset() {
      stop();
      last = null;
      at = Number.NEGATIVE_INFINITY;
    },
  };
}
