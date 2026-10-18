import { useCallback, useSyncExternalStore } from "react";
import { type Async, useAsync } from "@/hooks/use-async";
import { useAppFocus } from "@/lib/app-focus";
import {
  cancelDownload,
  clearFailure,
  downloadState,
  startDownload,
  subscribeDownload,
} from "@/lib/detector-download";
import {
  type DetectorsProgress,
  type DetectorsStatus,
  detectorsStatus,
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

/** Detector support status plus the shared download (progress, cancel) and remove. */
export function useDetectorSupport(): DetectorSupport {
  const status = useAsync(detectorsStatus, []);
  const { downloading, progress, failure } = useSyncExternalStore(subscribeDownload, downloadState);
  const { reload } = status;
  useAppFocus(reload);
  useStorageChanged(reload);

  const remove = useCallback(async () => {
    try {
      return await removeDetectors();
    } finally {
      clearFailure();
      storageChanged();
    }
  }, []);

  return {
    status: status.state,
    progress,
    downloading,
    failure,
    download: startDownload,
    cancel: cancelDownload,
    remove,
  };
}
