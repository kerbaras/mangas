import { createContext, type ReactNode, useContext, useEffect, useState } from "react";
import { downloadLabel } from "@/lib/download-labels";
import { type DownloadProgress, onDownloadProgress } from "@/lib/ipc";
import { notifyError } from "@/lib/notify";

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
      if (p.error)
        notifyError(`${downloadLabel(p.chapter_id)} didn't download`, p.error, {
          id: `download-${p.chapter_id}`,
        });
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
