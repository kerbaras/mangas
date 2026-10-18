import {
  createContext,
  type ReactNode,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
} from "react";
import {
  downloadContext,
  downloadLabel,
  failureLog,
  rememberDownloads,
} from "@/lib/download-labels";
import {
  applyEvents,
  applySnapshot,
  emptyQueue,
  jobsByChapter,
  type QueueState,
} from "@/lib/download-queue";
import { type DownloadProgress, downloadChapters } from "@/lib/ipc";
import {
  type DownloadJob,
  isActive,
  isFinalError,
  listDownloads,
  onQueueEvent,
  type QueueEvent,
  queueStatus,
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
  const retry =
    context && !isFinalError(error)
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

const QueueContext = createContext<QueueState>(emptyQueue);
const DownloadsContext = createContext<DownloadMap>(new Map());
type Forget = (match: (chapterId: string) => boolean) => void;
const ForgetContext = createContext<Forget>(() => {});

/** Progress events come several times a second per job; the UI folds them at most this often. */
export const BATCH_MS = 100;

/** Jobs from earlier runs carry their own label, so their toasts and logs name the chapter. */
function remember(job: DownloadJob) {
  if (downloadContext(job.chapter_id)) return;
  const label = job.manga_title
    ? `Chapter ${job.number} of ${job.manga_title}`
    : `Chapter ${job.number}`;
  rememberDownloads([[job.chapter_id, { label, source: job.source, mangaId: job.manga_id }]]);
}

/** App-wide so progress survives leaving and re-entering a manga's chapters. Seeded from the
 * persistent queue, so jobs left from the last run (or queued by the CLI) show up too. */
export function DownloadsProvider({ children }: { children: ReactNode }) {
  const [queue, setQueue] = useState<QueueState>(emptyQueue);
  const [forgotten, setForgotten] = useState<ReadonlySet<string>>(new Set());

  useEffect(() => {
    let unlisten: (() => void) | null = null;
    let alive = true;
    // Events arrive in order and win over the startup snapshot, which can resolve after them.
    const fromEvents = { jobs: new Set<number>(), gate: false };
    let pending: [QueueEvent, number][] = [];
    let timer: ReturnType<typeof setTimeout> | null = null;
    const flush = () => {
      if (timer) clearTimeout(timer);
      timer = null;
      const batch = pending;
      pending = [];
      if (batch.length && alive) setQueue((q) => applyEvents(q, batch));
    };
    onQueueEvent((e) => {
      if (e.kind === "gate") fromEvents.gate = true;
      else fromEvents.jobs.add(e.kind === "removed" ? e.id : e.job.id);
      if (e.kind !== "removed" && e.kind !== "gate") {
        remember(e.job);
        const chapter = e.job.chapter_id;
        setForgotten((f) => (f.has(chapter) ? new Set([...f].filter((c) => c !== chapter)) : f));
      }
      pending.push([e, Date.now()]);
      timer ??= setTimeout(flush, BATCH_MS);
      if (e.kind === "failed") chapterFailed(e.job.chapter_id, e.job.error ?? "failed");
    }).then((u) => {
      if (alive) unlisten = u;
      else u();
    });
    const warn = (e: unknown) => logUiWarning(`download queue: ${String(e)}`);
    listDownloads()
      .then((jobs) => {
        if (!alive) return;
        jobs.forEach(remember);
        flush();
        setQueue((q) => applySnapshot(q, { jobs }, fromEvents));
      })
      .catch(warn);
    queueStatus()
      .then((gate) => {
        if (alive) setQueue((q) => applySnapshot(q, { gate }, fromEvents));
      })
      .catch(warn);
    return () => {
      alive = false;
      if (timer) clearTimeout(timer);
      unlisten?.();
    };
  }, []);

  const progress = useMemo(() => {
    const out = new Map<string, DownloadProgress>();
    for (const [chapter, job] of jobsByChapter(queue.jobs))
      if (!forgotten.has(chapter)) out.set(chapter, toDownloadProgress(job));
    return out;
  }, [queue.jobs, forgotten]);

  const forget = useCallback<Forget>(
    (match) => setForgotten((f) => new Set([...f, ...[...progress.keys()].filter(match)])),
    [progress],
  );

  return (
    <QueueContext.Provider value={queue}>
      <ForgetContext.Provider value={forget}>
        <DownloadsContext.Provider value={progress}>{children}</DownloadsContext.Provider>
      </ForgetContext.Provider>
    </QueueContext.Provider>
  );
}

/** Each chapter's latest download, for chapter rows. */
export const useDownloads = () => useContext(DownloadsContext);
/** Every job, the queue-wide gate and recent progress, for the Downloads screen. */
export const useQueue = () => useContext(QueueContext);
/** Drops finished progress for chapters whose files were deleted, so they stop showing "On disk". */
export const useForgetDownloads = () => useContext(ForgetContext);

export const activeCount = (m: DownloadMap) => [...m.values()].filter(isActive).length;
