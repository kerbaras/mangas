import { type Async, useAsync } from "@/hooks/use-async";
import { useDebounced } from "@/hooks/use-debounced";
import { type ExportPick, exportTitle } from "@/lib/export-name";
import { type BundleMode, groupingOf } from "@/lib/export-plan";
import { exportFileName, planVolumes, type VolumePlan } from "@/lib/ipc.volumes";

/** Typing in the title or size fields replans once the user pauses. */
const PLAN_DELAY_MS = 300;

type Keyed = { key: string; data: VolumePlan[] };

/** The last plans that arrived; they stay on screen while a newer request is in flight. */
const settled = (state: Async<Keyed>): Async<VolumePlan[]> =>
  state.kind === "ready" ? { kind: "ready", data: state.data.data } : state;

/** Chapters from several series: one book, the only bundle `export_chapters` writes. */
export const joinedPlan = async (picks: ExportPick[], title: string): Promise<VolumePlan> => {
  const name = title.trim() || exportTitle(picks);
  return {
    number: null,
    title: name,
    file_name: await exportFileName(name),
    chapters: picks.flatMap((p) => p.chapters),
  };
};

type Input = { open: boolean; picks: ExportPick[]; mode: BundleMode; size: number; title: string };

/** The books the dialog's choices would write, planned by the backend for one series. */
export function useExportPlan({ open, picks, mode, size, title }: Input) {
  const planSize = useDebounced(size, PLAN_DELAY_MS);
  const planTitle = useDebounced(title, PLAN_DELAY_MS);
  const joined = picks.length > 1;
  const ids = picks.flatMap((p) => p.chapters.map((c) => c.id));
  const keyOf = (n: number, t: string) => [open, ids.join(), mode, n, t].join("|");
  const key = keyOf(planSize, planTitle);
  const planned = useAsync(async () => {
    const [pick] = picks;
    const data =
      joined || !pick
        ? [await joinedPlan(picks, planTitle)]
        : await planVolumes({
            source: pick.manga.source,
            mangaId: pick.manga.id,
            chapterIds: ids,
            grouping: groupingOf(mode, planSize),
            title: planTitle,
          });
    return { key, data };
  }, [key]);
  const fresh = planned.state.kind === "ready" && planned.state.data.key === keyOf(size, title);
  return { plans: settled(planned.state), fresh, joined };
}
