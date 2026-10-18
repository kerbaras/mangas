import { errorMessage } from "@/lib/format";
import {
  cancelDetectorsDownload,
  type DetectorsProgress,
  downloadDetectors,
  onDetectorsProgress,
} from "@/lib/ipc.detectors";
import { storageChanged } from "@/lib/storage-changed";

/** The app's one detector download, kept outside React so it outlives the view that started it. */
export type DownloadState = {
  downloading: boolean;
  /** Null until the first progress event. */
  progress: DetectorsProgress | null;
  /** Why the last download failed; a cancel is not a failure. */
  failure: string | null;
};

let state: DownloadState = { downloading: false, progress: null, failure: null };
let cancelled = false;
const listeners = new Set<() => void>();

function set(next: Partial<DownloadState>) {
  state = { ...state, ...next };
  for (const l of listeners) l();
}

export const downloadState = () => state;

export function subscribeDownload(listener: () => void) {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

export async function startDownload() {
  if (state.downloading) return;
  cancelled = false;
  set({ downloading: true, progress: null, failure: null });
  let unlisten: (() => void) | undefined;
  try {
    unlisten = await onDetectorsProgress((progress) => set({ progress }));
    // Cancel clicked while the listener was registering: never start.
    if (cancelled) return;
    await downloadDetectors();
  } catch (e: unknown) {
    if (!cancelled) set({ failure: errorMessage(e) });
  } finally {
    unlisten?.();
    set({ downloading: false, progress: null });
    storageChanged();
  }
}

export function cancelDownload() {
  cancelled = true;
  void cancelDetectorsDownload();
}

export function clearFailure() {
  set({ failure: null });
}
