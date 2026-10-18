import { createContext, type ReactNode, useCallback, useContext, useEffect, useState } from "react";
import {
  downloadContext,
  downloadLabel,
  failureLog,
  rememberDownloads,
} from "@/lib/download-labels";
import { type DownloadProgress, downloadChapters } from "@/lib/ipc";
import {
  type DownloadJob,
  isActive,
  listDownloads,
  onQueueEvent,
  toDownloadProgress,
} from "@/lib/ipc.downloads";
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
type Forget = (match: (chapterId: string) => boolean) => void;
const ForgetContext = createContext<Forget>(() => {});

/** Jobs from earlier runs carry their own label, so their toasts and logs name the chapter. */
function remember(job: DownloadJob) {
  if (downloadContext(job.chapter_id)) return;
  const label = job.manga_title
    ? `Chapter ${job.number} of ${job.manga_title}`
    : `Chapter ${job.number}`;
  rememberDownloads([[job.chapter_id, { label, source: job.source, mangaId: job.manga_id }]]);
}

/** App-wide so progress survives leaving and re-entering a manga's chapters. Seeded from the
 * persistent queue, so jobs left from the last run show up after a restart. */
export function DownloadsProvider({ children }: { children: ReactNode }) {
  const [progress, setProgress] = useState<Map<string, DownloadProgress>>(new Map());

  useEffect(() => {
    let unlisten: (() => void) | null = null;
    let alive = true;
    const ids = new Map<number, string>();
    // Events arrive in order; the startup snapshot can resolve after one, so it never
    // replaces a job an event already reported.
    const fromEvents = new Set<number>();
    const put = (job: DownloadJob, snapshot = false) => {
      if (snapshot && fromEvents.has(job.id)) return;
      if (!snapshot) fromEvents.add(job.id);
      ids.set(job.id, job.chapter_id);
      remember(job);
      setProgress((m) => new Map(m).set(job.chapter_id, toDownloadProgress(job)));
    };
    onQueueEvent((e) => {
      if (e.kind === "removed") {
        const chapter = ids.get(e.id);
        fromEvents.add(e.id);
        if (chapter) setProgress((m) => (m.delete(chapter) ? new Map(m) : m));
        return;
      }
      put(e.job);
      if (e.kind === "failed") chapterFailed(e.job.chapter_id, e.job.error ?? "failed");
    }).then((u) => {
      if (alive) unlisten = u;
      else u();
    });
    listDownloads()
      .then((jobs) => {
        if (alive) for (const job of jobs) put(job, true);
      })
      .catch((e: unknown) => logUiWarning(`download queue: ${String(e)}`));
    return () => {
      alive = false;
      unlisten?.();
    };
  }, []);

  const forget = useCallback<Forget>(
    (match) => setProgress((m) => new Map([...m].filter(([id]) => !match(id)))),
    [],
  );

  return (
    <ForgetContext.Provider value={forget}>
      <DownloadsContext.Provider value={progress}>{children}</DownloadsContext.Provider>
    </ForgetContext.Provider>
  );
}

export const useDownloads = () => useContext(DownloadsContext);
/** Drops finished progress for chapters whose files were deleted, so they stop showing "On disk". */
export const useForgetDownloads = () => useContext(ForgetContext);

export const activeCount = (m: DownloadMap) => [...m.values()].filter(isActive).length;
