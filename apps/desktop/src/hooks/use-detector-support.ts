import { useCallback, useRef, useState } from "react";
import { type Async, useAsync } from "@/hooks/use-async";
import { useAppFocus } from "@/lib/app-focus";
import { errorMessage } from "@/lib/format";
import {
  cancelDetectorsDownload,
  type DetectorsProgress,
  type DetectorsStatus,
  detectorsStatus,
  downloadDetectors,
  onDetectorsProgress,
  removeDetectors,
} from "@/lib/ipc.detectors";
import { storageChanged, useStorageChanged } from "@/lib/storage-changed";

export type DetectorSupport = {
  status: Async<DetectorsStatus>;
  /** Set while a download runs (null until the first progress event). */
  progress: DetectorsProgress | null;
  downloading: boolean;
  /** Why the last download failed; a cancel is not a failure. */
  failure: string | null;
  download: () => Promise<void>;
  cancel: () => void;
  /** Resolves to the bytes freed. */
  remove: () => Promise<number>;
};

/** Detector support status plus download (progress, cancel) and remove. */
export function useDetectorSupport(): DetectorSupport {
  const status = useAsync(detectorsStatus, []);
  const [progress, setProgress] = useState<DetectorsProgress | null>(null);
  const [downloading, setDownloading] = useState(false);
  const [failure, setFailure] = useState<string | null>(null);
  const cancelled = useRef(false);
  const running = useRef(false);
  const { patch, reload } = status;
  useAppFocus(reload);
  useStorageChanged(reload);

  const download = useCallback(async () => {
    if (running.current) return;
    running.current = true;
    cancelled.current = false;
    setFailure(null);
    setProgress(null);
    setDownloading(true);
    let unlisten: (() => void) | undefined;
    try {
      unlisten = await onDetectorsProgress(setProgress);
      const next = await downloadDetectors();
      patch(() => next);
      storageChanged();
    } catch (e: unknown) {
      if (!cancelled.current) setFailure(errorMessage(e));
      reload();
    } finally {
      unlisten?.();
      running.current = false;
      setDownloading(false);
      setProgress(null);
    }
  }, [patch, reload]);

  const cancel = useCallback(() => {
    cancelled.current = true;
    void cancelDetectorsDownload();
  }, []);

  const remove = useCallback(async () => {
    try {
      return await removeDetectors();
    } finally {
      setFailure(null);
      storageChanged();
    }
  }, []);

  return { status: status.state, progress, downloading, failure, download, cancel, remove };
}
