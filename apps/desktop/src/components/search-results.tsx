import { ProviderChips } from "@/components/provider-chips";
import { type ResultActions, SearchGroupView } from "@/components/search-group";
import type { ProviderInfo } from "@/lib/ipc";
import { type GroupState, summary } from "@/lib/search-groups";

type Props = ResultActions & {
  providers: ProviderInfo[];
  groups: Record<string, GroupState>;
  filter: string;
  onFilter: (filter: string) => void;
  onRetry: (id: string) => void;
};

/** Filter chips, a one-line summary and a group per provider (just one when filtered). */
export function SearchResults({ providers, groups, filter, onFilter, onRetry, ...actions }: Props) {
  const visible = filter === "all" ? providers : providers.filter((p) => p.id === filter);
  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-col gap-2">
        <ProviderChips providers={providers} groups={groups} value={filter} onChange={onFilter} />
        <p className="text-caption text-ink-3" aria-live="polite">
          {summary(providers.map((p) => groups[p.id]))}
        </p>
      </div>
      {visible.map((p) => (
        <SearchGroupView
          key={p.id}
          provider={p}
          state={groups[p.id]}
          filtered={filter !== "all"}
          onRetry={() => onRetry(p.id)}
          onShowAll={() => onFilter(p.id)}
          {...actions}
        />
      ))}
    </div>
  );
}
