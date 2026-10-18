import { useCallback, useRef, useState } from "react";
import { type Async, useAsync } from "@/hooks/use-async";
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
  const { patch, reload } = status;

  const download = useCallback(async () => {
    cancelled.current = false;
    setFailure(null);
    setProgress(null);
    let unlisten: (() => void) | undefined;
    try {
      unlisten = await onDetectorsProgress(setProgress);
      setDownloading(true);
      const next = await downloadDetectors();
      patch(() => next);
    } catch (e: unknown) {
      if (!cancelled.current) setFailure(errorMessage(e));
      reload();
    } finally {
      unlisten?.();
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
      reload();
    }
  }, [reload]);

  return { status: status.state, progress, downloading, failure, download, cancel, remove };
}
