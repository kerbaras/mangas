import { AlertTriangle, Hourglass } from "lucide-react";
import type { ReactNode } from "react";
import { ProviderIcon } from "@/components/provider-icon";
import { useT } from "@/i18n/use-t";
import { cn } from "@/lib/cn";
import type { ProviderInfo } from "@/lib/ipc";
import { type Chip, chipOf, type GroupState } from "@/lib/search-groups";

type Props = {
  providers: ProviderInfo[];
  groups: Record<string, GroupState>;
  value: string;
  onChange: (filter: string) => void;
};

/** "All" plus one chip per searched provider; each shows its count, a still hourglass or a warning. */
export function ProviderChips({ providers, groups, value, onChange }: Props) {
  const t = useT();
  const total = providers.reduce((n, p) => n + (chipOf(groups[p.id]).count ?? 0), 0);
  const anyDone = providers.some((p) => groups[p.id]?.kind === "done");
  return (
    <fieldset aria-label={t("search.filterByProvider")} className="flex flex-wrap gap-1.5">
      <ChipButton on={value === "all"} onClick={() => onChange("all")}>
        {t("search.all")}
        {anyDone && <span className="tabular-nums text-ink-3">{total}</span>}
      </ChipButton>
      {providers.map((p) => (
        <ChipButton key={p.id} on={value === p.id} onClick={() => onChange(p.id)}>
          <ProviderIcon provider={p} />
          {p.name}
          <Trailing chip={chipOf(groups[p.id])} />
        </ChipButton>
      ))}
    </fieldset>
  );
}

function Trailing({ chip }: { chip: Chip }) {
  const t = useT();
  if (chip.searching)
    return <Hourglass aria-label={t("search.searching")} className="size-3 text-ink-3" />;
  if (chip.problem)
    return <AlertTriangle aria-label={t("search.couldntSearch")} className="size-3 text-warn" />;
  if (chip.count === null) return null;
  return <span className="tabular-nums text-ink-3">{chip.count}</span>;
}

type ChipProps = { on: boolean; onClick: () => void; children: ReactNode };

function ChipButton({ on, onClick, children }: ChipProps) {
  return (
    <button
      type="button"
      aria-pressed={on}
      onClick={onClick}
      className={cn(
        "inline-flex h-7 items-center gap-1.5 rounded-full border px-2.5 text-caption transition-colors",
        on
          ? "border-accent bg-accent-soft text-ink"
          : "border-stroke text-ink-2 hover:bg-surface-2",
      )}
    >
      {children}
    </button>
  );
}
