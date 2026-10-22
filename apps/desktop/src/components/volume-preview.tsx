import { BookOpen } from "lucide-react";
import { Hint } from "@/components/hint";
import type { Async } from "@/hooks/use-async";
import type { BookBudget, VolumePlan } from "@/lib/ipc.volumes";
import { againstBudget } from "@/lib/kindle-size";
import { planSummary } from "@/lib/volume-plan";

type Props = {
  plans: Async<VolumePlan[]>;
  /** Each book's estimate against its budget, in plan order (`kindle::plan_budget`). */
  budgets?: (BookBudget | null)[] | null;
  folder: string;
  /** The plans are for an older selection and a new plan is on its way. */
  stale?: boolean;
  /** Why the export would be refused before it writes anything (`couldn't list <folder>: …`). */
  blocked?: string | null;
};

/** The EPUB files an export will write, one row each. */
export function VolumePreview({ plans, budgets, folder, stale = false, blocked = null }: Props) {
  if (plans.kind === "loading") return <p className="text-caption text-ink-3">Planning books…</p>;
  if (plans.kind === "error")
    return (
      <p className="text-caption text-accent-text [overflow-wrap:anywhere]" role="alert">
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
        {plans.data.map((p, i) => (
          <li
            key={p.chapters.map((c) => c.id).join(" ")}
            className="flex items-center gap-3 border-b border-stroke px-3 py-2 last:border-b-0"
          >
            <BookOpen className="size-4 shrink-0 text-ink-3" aria-hidden />
            <Hint hint={p.file_name} truncated>
              <span className="min-w-0 flex-1 truncate font-mono text-caption">{p.file_name}</span>
            </Hint>
            <span className="shrink-0 text-caption text-ink-3 tabular-nums">{planSummary(p)}</span>
            <BookSize budget={budgets?.[i] ?? null} />
          </li>
        ))}
      </ul>
      <Hint hint={folder} truncated>
        <p className="truncate text-caption text-ink-3">Saved to {folder}</p>
      </Hint>
      {blocked && (
        <p className="text-caption text-accent-text [overflow-wrap:anywhere]" role="alert">
          {blocked}
        </p>
      )}
    </div>
  );
}

/** "212 MB of 200 MB", in the warning colour when the book is over its budget. */
function BookSize({ budget }: { budget: BookBudget | null }) {
  if (!budget) return null;
  const tone = budget.over_budget ? "text-accent-text" : "text-ink-3";
  return (
    <span className={`shrink-0 text-caption tabular-nums ${tone}`}>
      {againstBudget(budget)}
      {budget.over_budget && <span className="sr-only">, over budget</span>}
    </span>
  );
}
