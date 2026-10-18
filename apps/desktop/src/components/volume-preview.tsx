import { BookOpen } from "lucide-react";
import type { Async } from "@/hooks/use-async";
import type { VolumePlan } from "@/lib/ipc.volumes";
import { planSummary } from "@/lib/volume-plan";

type Props = {
  plans: Async<VolumePlan[]>;
  folder: string;
  /** The plans are for an older selection and a new plan is on its way. */
  stale?: boolean;
};

/** The EPUB files an export will write, one row each. */
export function VolumePreview({ plans, folder, stale = false }: Props) {
  if (plans.kind === "loading") return <p className="text-caption text-ink-3">Planning books…</p>;
  if (plans.kind === "error")
    return (
      <p className="text-caption text-accent" role="alert">
        Couldn't plan the books: {plans.message}
      </p>
    );
  if (plans.data.length === 0)
    return <p className="text-caption text-ink-3">None of these chapters is downloaded.</p>;
  return (
    <div
      className={`flex flex-col gap-1.5 transition-opacity ${stale ? "opacity-60" : ""}`}
      aria-busy={stale}
    >
      <ul
        aria-label="Files to write"
        className="max-h-56 overflow-y-auto rounded-md border border-stroke"
      >
        {plans.data.map((p) => (
          <li
            key={p.file_name}
            className="flex items-center gap-3 border-b border-stroke px-3 py-2 last:border-b-0"
          >
            <BookOpen className="size-4 shrink-0 text-ink-3" aria-hidden />
            <span className="min-w-0 flex-1 truncate font-mono text-caption" title={p.file_name}>
              {p.file_name}
            </span>
            <span className="shrink-0 text-caption text-ink-3 tabular-nums">{planSummary(p)}</span>
          </li>
        ))}
      </ul>
      <p className="truncate text-caption text-ink-3" title={folder}>
        Saved to {folder}
      </p>
    </div>
  );
}
