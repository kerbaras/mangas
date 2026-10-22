import { type Async, useAsync } from "@/hooks/use-async";
import type { Preset } from "@/lib/ipc";
import type { ExportResult, Oversized } from "@/lib/ipc.export";
import { fitLevers, type Levers, type VolumePlan } from "@/lib/ipc.volumes";
import { combinedLevers, oversizedDirs, oversizedOf } from "@/lib/kindle-size";

type Choice = {
  device: string;
  preset: Preset;
  quality: number | null;
  email: boolean;
  splitMb: number | null;
  /** The dialog can put fewer chapters in each book (one series): else that lever is dropped. */
  bundle: boolean;
};

/**
 * The written files over the delivery's budget, and what makes their books fit as their plan
 * offers it (`kindle::book_levers`): levers null while there are none, loading or failed.
 */
export function useOversized(
  results: ExportResult[],
  plans: Async<VolumePlan[]>,
  choice: Choice,
): { files: Oversized[]; levers: Levers | null } {
  const { device, preset, quality, email, splitMb, bundle } = choice;
  const books = plans.kind === "ready" ? oversizedDirs(results, plans.data, email) : [];
  const key = JSON.stringify([books, device, preset, quality, email, splitMb]);
  const fit = useAsync(async () => {
    if (books.length === 0 || !device) return null;
    const levers = await fitLevers({ books, device, preset, quality, email, splitMb });
    return { key, levers: combinedLevers(levers) };
  }, [key]);
  const s = fit.state;
  const levers = s.kind === "ready" && s.data?.key === key ? s.data.levers : null;
  const files = oversizedOf(results, email);
  return { files, levers: levers && !bundle ? { ...levers, chapters_per_book: null } : levers };
}
