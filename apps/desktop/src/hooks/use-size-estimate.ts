import { type Async, useAsync } from "@/hooks/use-async";
import { useDebounced } from "@/hooks/use-debounced";
import type { Preset } from "@/lib/ipc";
import { type Estimate, estimateExport, type VolumePlan } from "@/lib/ipc.volumes";
import { downloadedBooks } from "@/lib/kindle-size";

/** Encoding sample pages costs a second or two: wait until the choices settle. */
const ESTIMATE_DELAY_MS = 600;

type Input = {
  plans: Async<VolumePlan[]>;
  device: string;
  preset: Preset;
  quality: number | null;
};

/**
 * About how big the largest planned book will be with the dialog's device, smart cut and
 * quality. Null while there is nothing to measure or the estimate failed (no warning then).
 */
export function useSizeEstimate({ plans, device, preset, quality }: Input): Estimate | null {
  const books = plans.kind === "ready" ? downloadedBooks(plans.data) : [];
  const key = JSON.stringify([books, device, preset, quality]);
  const settled = useDebounced(key, ESTIMATE_DELAY_MS);
  const estimate = useAsync(async () => {
    if (books.length === 0 || !device || settled !== key) return null;
    return { key, data: await estimateExport({ books, device, preset, quality }) };
  }, [settled]);
  const s = estimate.state;
  return s.kind === "ready" && s.data?.key === key ? s.data.data : null;
}
