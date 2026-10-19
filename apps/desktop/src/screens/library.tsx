import { Library, Plus, Search } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { CategoriesDialog } from "@/components/categories-dialog";
import { CheckUpdatesButton } from "@/components/check-updates-button";
import { EmptyState } from "@/components/empty-state";
import type { EntryHandlers } from "@/components/entry-menu";
import { ErrorAlert } from "@/components/error-alert";
import { ImportMenu } from "@/components/import-menu";
import { LibraryList } from "@/components/library-list";
import { LibraryToolbar } from "@/components/library-toolbar";
import { MangaCard } from "@/components/manga-card";
import { ScreenHeader } from "@/components/screen-header";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { useImport } from "@/hooks/use-import";
import { useLibraryView } from "@/hooks/use-library-view";
import { useShelf } from "@/hooks/use-shelf";
import { mangaKey } from "@/lib/format";
import type { Category } from "@/lib/ipc.library";
import { filterEntries, liveView, sortEntries } from "@/lib/library-view";
import { useNavigation } from "@/lib/navigation";
import { notifyError } from "@/lib/notify";
import { useUpdates } from "@/lib/updates";

const noCategories: Category[] = [];

export function LibraryScreen() {
  const nav = useNavigation();
  const shelf = useShelf();
  const { state, reload } = shelf;
  const imports = useImport(reload);
  const [query, setQuery] = useState("");
  const [stored, setView] = useLibraryView();
  const [editing, setEditing] = useState(false);
  const { found } = useUpdates();
  useEffect(() => {
    if (found > 0) reload();
  }, [found, reload]);

  const entries = state.kind === "ready" ? state.data.entries : [];
  const categories = state.kind === "ready" ? state.data.categories : noCategories;
  const view = liveView(
    stored,
    entries,
    categories.map((c) => c.id),
  );
  const items = useMemo(
    () => sortEntries(filterEntries(entries, view, query), view.sort),
    [entries, view, query],
  );

  const handlers: EntryHandlers = {
    onOpen: (e) => nav.go({ screen: "manga", manga: e }),
    onUnfollow: (e) =>
      shelf.unfollow(e).catch((err: unknown) => notifyError(`Couldn't unfollow ${e.title}`, err)),
    onStatus: (e, status) =>
      shelf.setStatus(e, status).catch((err: unknown) => notifyError("Couldn't set status", err)),
    onToggleCategory: (e, id, on) =>
      shelf
        .toggleCategory(e, id, on)
        .catch((err: unknown) => notifyError("Couldn't change categories", err)),
    onEditCategories: () => setEditing(true),
  };
  const count = state.kind === "ready" ? entries.length : null;
  const filtered = query.trim() !== "" || view.category !== null || !!view.source || !!view.status;

  return (
    <section className="flex h-full flex-col">
      <ScreenHeader title="Library" subtitle={count !== null && `${count} followed`}>
        {count ? (
          <div className="relative">
            <Search className="pointer-events-none absolute top-2 left-2 size-4 text-ink-3" />
            <Input
              type="search"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder="Filter by title"
              aria-label="Filter by title"
              className="w-56 pl-7"
            />
          </div>
        ) : null}
        {count ? <CheckUpdatesButton /> : null}
        <ImportMenu busy={imports.busy} onPick={imports.start} />
        <Button variant="primary" size="sm" onClick={() => nav.go({ screen: "search" })}>
          <Plus /> Follow a manga
        </Button>
      </ScreenHeader>
      {count !== null ? (
        <LibraryToolbar
          view={view}
          entries={entries}
          categories={categories}
          onChange={setView}
          onEditCategories={() => setEditing(true)}
        />
      ) : null}
      <div className="relative min-h-0 flex-1 overflow-auto overscroll-contain">
        {state.kind === "loading" && <p className="p-4 text-ink-3">Loading…</p>}
        {state.kind === "error" && (
          <div className="p-4">
            <ErrorAlert
              title="Couldn't open the library"
              message={state.message}
              onRetry={reload}
            />
          </div>
        )}
        {state.kind === "ready" && entries.length === 0 && (
          <EmptyState
            icon={<Library />}
            title="Nothing on the shelf yet"
            action={
              <Button variant="primary" onClick={() => nav.go({ screen: "search" })}>
                <Search /> Search a source
              </Button>
            }
          >
            Search a source and follow a title, or import chapter folders and CBZ files you already
            have. Chapters show up here, ready to export to your e-reader.
          </EmptyState>
        )}
        {entries.length > 0 && items.length === 0 && filtered && (
          <EmptyState title="Nothing matches">
            {query.trim()
              ? `No title here matches “${query.trim()}”. Try a shorter title, or clear the filter.`
              : "No title here has this category, source or status. Pick another or clear the filters."}
          </EmptyState>
        )}
        {items.length > 0 && view.layout === "grid" && (
          <ul className="grid grid-cols-[repeat(auto-fill,minmax(148px,1fr))] gap-3 p-4">
            {items.map((e) => (
              <li key={mangaKey(e)}>
                <MangaCard entry={e} categories={categories} {...handlers} />
              </li>
            ))}
          </ul>
        )}
        {items.length > 0 && view.layout === "list" && (
          <LibraryList entries={items} categories={categories} {...handlers} />
        )}
      </div>
      <CategoriesDialog
        open={editing}
        onOpenChange={setEditing}
        categories={categories}
        onCreate={shelf.addCategory}
        onRename={shelf.renameCategory}
        onDelete={async (id) => {
          await shelf.removeCategory(id);
          if (stored.category === id) setView({ category: null });
        }}
      />
    </section>
  );
}
