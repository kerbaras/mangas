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
import { clearCache } from "@/lib/ipc.storage";
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
  /** Empties the detector folder, like Storage → Delete detector support; the bytes freed. */
  removeLeftovers: () => Promise<number>;
};

/** Detector support status plus the shared download (progress, cancel) and remove. */
export function useDetectorSupport(): DetectorSupport {
  const status = useAsync(detectorsStatus, []);
  const { downloading, progress, failure } = useSyncExternalStore(subscribeDownload, downloadState);
  const { reload } = status;
  useAppFocus(reload);
  useStorageChanged(reload);

  const remove = useCallback(() => afterRemove(removeDetectors()), []);
  const removeLeftovers = useCallback(
    () => afterRemove(clearCache("models").then((c) => c.bytes)),
    [],
  );

  return {
    status: status.state,
    progress,
    downloading,
    failure,
    download: startDownload,
    cancel: cancelDownload,
    remove,
    removeLeftovers,
  };
}

async function afterRemove(removing: Promise<number>): Promise<number> {
  try {
    return await removing;
  } finally {
    clearFailure();
    storageChanged();
  }
}
