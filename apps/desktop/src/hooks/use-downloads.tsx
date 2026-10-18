import { createContext, type ReactNode, useContext, useEffect, useState } from "react";
import { downloadContext, downloadLabel, failureLog } from "@/lib/download-labels";
import { type DownloadProgress, downloadChapters, onDownloadProgress } from "@/lib/ipc";
import { notifyError } from "@/lib/notify";
import { logUiWarning } from "@/lib/report";

/**
 * One chapter failed: a warning in the log with its ids, and a toast that retries it, or offers
 * Connect first when the provider asked for a browser check.
 */
export function chapterFailed(id: string, error: string) {
  const context = downloadContext(id);
  logUiWarning(failureLog(id, error, context));
  const retry = context
    ? () =>
        void downloadChapters(context.source, context.mangaId, [id]).catch((e: unknown) =>
          notifyError(`${context.label} didn't download`, e, { id: `download-${id}` }),
        )
    : undefined;
  notifyError(`${downloadLabel(id)} didn't download`, error, {
    id: `download-${id}`,
    source: context?.source,
    retry,
  });
}

export type DownloadMap = ReadonlyMap<string, DownloadProgress>;

const DownloadsContext = createContext<DownloadMap>(new Map());

/** App-wide so progress survives leaving and re-entering a manga's chapters. */
export function DownloadsProvider({ children }: { children: ReactNode }) {
  const [progress, setProgress] = useState<Map<string, DownloadProgress>>(new Map());

  useEffect(() => {
    let unlisten: (() => void) | null = null;
    let alive = true;
    onDownloadProgress((p) => {
      setProgress((m) => new Map(m).set(p.chapter_id, p));
      if (p.error) chapterFailed(p.chapter_id, p.error);
    }).then((u) => {
      if (alive) unlisten = u;
      else u();
    });
    return () => {
      alive = false;
      unlisten?.();
    };
  }, []);

  return <DownloadsContext.Provider value={progress}>{children}</DownloadsContext.Provider>;
}

export const useDownloads = () => useContext(DownloadsContext);

export const activeCount = (m: DownloadMap) => [...m.values()].filter((p) => !p.done).length;
