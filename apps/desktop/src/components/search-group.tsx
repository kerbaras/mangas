import { Link2, Loader2 } from "lucide-react";
import type { ReactNode } from "react";
import { RetryButton } from "@/components/error-alert";
import { Hint } from "@/components/hint";
import { ProviderIcon } from "@/components/provider-icon";
import { ProviderStatusBadge } from "@/components/provider-status-badge";
import { SearchResult } from "@/components/search-result";
import { Button } from "@/components/ui/button";
import { describeFailure } from "@/lib/errors";
import { mangaKey } from "@/lib/format";
import type { Manga, ProviderInfo } from "@/lib/ipc";
import { isFinalError } from "@/lib/ipc.downloads";
import { connectThen } from "@/lib/notify";
import { forgetStatus } from "@/lib/provider-status";
import { type GroupState, shown } from "@/lib/search-groups";

export type ResultActions = {
  followed: Set<string>;
  busy: string | null;
  onFollow: (m: Manga) => void;
  onOpen: (m: Manga) => void;
};

type Props = ResultActions & {
  provider: ProviderInfo;
  state: GroupState | undefined;
  /** Filtered to this provider: show every result and no "Show all". */
  filtered: boolean;
  onRetry: () => void;
  onShowAll: () => void;
};

/** One provider's results under its name, or why it has none and what to do about it. */
export function SearchGroupView({
  provider: p,
  state,
  filtered,
  onRetry,
  onShowAll,
  ...rest
}: Props) {
  const group = state?.kind === "done" ? state.group : null;
  const items = group?.status.state === "ready" ? group.items : [];
  const more = items.length - shown(items, filtered).length;
  const connect = () =>
    void connectThen(
      p.id,
      () => {
        forgetStatus(p.id);
        onRetry();
      },
      p.name,
    );
  return (
    <section aria-label={p.name} className="flex flex-col gap-1">
      <header className="flex h-8 items-center gap-2 px-2">
        <ProviderIcon provider={p} />
        <h2 className="text-body font-semibold">{p.name}</h2>
        {group && group.status.state !== "ready" && <ProviderStatusBadge status={group.status} />}
        {group?.status.state === "ready" && (
          <span className="text-caption text-ink-3 tabular-nums">{items.length}</span>
        )}
        {more > 0 && (
          <Button size="sm" variant="ghost" className="ml-auto" onClick={onShowAll}>
            Show all {items.length}
          </Button>
        )}
      </header>
      {!group && (
        <Note>
          <Loader2 className="size-3.5 animate-spin" /> Searching…
        </Note>
      )}
      {group?.status.state === "needs_session" && (
        <Note
          action={
            <Button size="sm" variant="primary" onClick={connect}>
              <Link2 /> Connect
            </Button>
          }
        >
          {p.name} asks for a browser check before it can search.
        </Note>
      )}
      {group?.status.state === "unreachable" && (
        <Unreachable name={p.name} reason={group.status.reason} onRetry={onRetry} />
      )}
      {group?.status.state === "ready" && items.length === 0 && <Note>No results here.</Note>}
      {items.length > 0 && (
        <ul className="flex flex-col gap-1">
          {shown(items, filtered).map((m) => (
            <SearchResult
              key={mangaKey(m)}
              manga={m}
              followed={rest.followed.has(mangaKey(m))}
              busy={rest.busy === mangaKey(m)}
              onFollow={rest.onFollow}
              onOpen={rest.onOpen}
            />
          ))}
        </ul>
      )}
    </section>
  );
}

function Unreachable(props: { name: string; reason: string; onRetry: () => void }) {
  const { label, tooltip } = describeFailure(props.reason);
  const retry = isFinalError(props.reason) ? undefined : <RetryButton onClick={props.onRetry} />;
  return (
    <Note action={retry}>
      <Hint hint={tooltip}>
        <span className="min-w-0 [overflow-wrap:anywhere]">
          Couldn't reach {props.name}: {label}
        </span>
      </Hint>
    </Note>
  );
}

function Note({ children, action }: { children: ReactNode; action?: ReactNode }) {
  return (
    <div className="flex min-h-10 items-center gap-3 rounded-md bg-surface-2/60 px-3 py-2 text-body text-ink-2">
      <div className="flex min-w-0 flex-1 items-center gap-2">{children}</div>
      {action}
    </div>
  );
}
