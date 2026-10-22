import { type Async, useAsync } from "@/hooks/use-async";
import { useDebounced } from "@/hooks/use-debounced";
import type { Preset } from "@/lib/ipc";
import type { ExportFormat } from "@/lib/ipc.export";
import { type BookBudget, estimateExport, type VolumePlan } from "@/lib/ipc.volumes";
import { alignBudgets, bookDirs } from "@/lib/kindle-size";

/** Encoding sample pages costs a second or two: wait until the choices settle. */
const ESTIMATE_DELAY_MS = 600;

type Input = {
  plans: Async<VolumePlan[]>;
  device: string;
  preset: Preset;
  quality: number | null;
  /** Emailed books are held to 50 MB, others to 200 MB. */
  email: boolean;
  splitMb: number | null;
  format: ExportFormat;
};

/**
 * Each planned book's estimate against its delivery's budget (`kindle::plan_budget`), in plan
 * order; null while there is nothing to measure or the estimate failed (no warning then).
 */
export function useSizeEstimate(input: Input): (BookBudget | null)[] | null {
  const { plans, ...choice } = input;
  const dirs = plans.kind === "ready" ? bookDirs(plans.data) : [];
  const books = dirs.filter((d) => d.length > 0);
  const key = JSON.stringify([dirs, choice]);
  const settled = useDebounced(key, ESTIMATE_DELAY_MS);
  const estimate = useAsync(async () => {
    if (books.length === 0 || !choice.device || settled !== key) return null;
    const budgets = await estimateExport({ ...choice, books });
    return { key, data: alignBudgets(dirs, budgets) };
  }, [settled]);
  const s = estimate.state;
  return s.kind === "ready" && s.data?.key === key ? s.data.data : null;
}
