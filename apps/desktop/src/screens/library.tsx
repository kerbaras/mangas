import { Plus, Search } from "lucide-react";
import { useMemo, useState } from "react";
import { EmptyState } from "@/components/empty-state";
import { ImportMenu } from "@/components/import-menu";
import { ImportStatusBar } from "@/components/import-status";
import { MangaCard } from "@/components/manga-card";
import { ScreenHeader } from "@/components/screen-header";
import { type LibrarySort, SortMenu, sortLibrary } from "@/components/sort-menu";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { useImport } from "@/hooks/use-import";
import { useLibrary } from "@/hooks/use-library";
import { mangaKey } from "@/lib/format";
import type { Manga } from "@/lib/ipc";
import { useNavigation } from "@/lib/navigation";

export function LibraryScreen() {
  const nav = useNavigation();
  const { state, reload, unfollow } = useLibrary();
  const imports = useImport(reload);
  const [filter, setFilter] = useState("");
  const [sort, setSort] = useState<LibrarySort>("title");

  const items = useMemo(() => {
    if (state.kind !== "ready") return [];
    const q = filter.trim().toLowerCase();
    return sortLibrary(
      state.data.filter((m) => !q || m.title.toLowerCase().includes(q)),
      sort,
    );
  }, [state, filter, sort]);

  const open = (m: Manga) => nav.go({ screen: "manga", manga: m });
  const count = state.kind === "ready" ? state.data.length : null;

  return (
    <section className="flex h-full flex-col">
      <ScreenHeader title="Library" subtitle={count !== null && `${count} followed`}>
        {count ? (
          <div className="relative">
            <Search className="pointer-events-none absolute top-2 left-2 size-4 text-ink-3" />
            <Input
              type="search"
              value={filter}
              onChange={(e) => setFilter(e.target.value)}
              placeholder="Filter by title"
              aria-label="Filter by title"
              className="w-56 pl-7"
            />
          </div>
        ) : null}
        {count ? <SortMenu value={sort} onChange={setSort} /> : null}
        <ImportMenu busy={imports.status.kind === "busy"} onPick={imports.start} />
        <Button variant="primary" size="sm" onClick={() => nav.go({ screen: "search" })}>
          <Plus /> Follow a manga
        </Button>
      </ScreenHeader>
      <ImportStatusBar status={imports.status} onDismiss={imports.dismiss} />
      <div className="flex-1 overflow-auto p-4">
        {state.kind === "loading" && <p className="text-ink-3">Loading…</p>}
        {state.kind === "error" && (
          <p className="text-accent">Couldn't open the library: {state.message}</p>
        )}
        {state.kind === "ready" && state.data.length === 0 && (
          <EmptyState
            title="Nothing on the shelf yet"
            action={
              <Button variant="primary" onClick={() => nav.go({ screen: "search" })}>
                <Search /> Search a source
              </Button>
            }
          >
            Follow a title from a source, or import chapter folders and CBZ files you already have.
            Chapters show up here, ready to send to your Kindle.
          </EmptyState>
        )}
        {state.kind === "ready" && state.data.length > 0 && items.length === 0 && (
          <EmptyState title={`Nothing matches “${filter.trim()}”`}>
            Try a shorter title, or clear the filter.
          </EmptyState>
        )}
        {items.length > 0 && (
          <ul className="grid grid-cols-[repeat(auto-fill,minmax(148px,1fr))] gap-3">
            {items.map((m) => (
              <li key={mangaKey(m)}>
                <MangaCard manga={m} onOpen={open} onUnfollow={unfollow} />
              </li>
            ))}
          </ul>
        )}
      </div>
    </section>
  );
}
