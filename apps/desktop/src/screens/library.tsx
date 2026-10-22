import { Library, Plus, Search } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { CategoriesDialog } from "@/components/categories-dialog";
import { CheckUpdatesButton } from "@/components/check-updates-button";
import { EmptyState } from "@/components/empty-state";
import type { EntryHandlers } from "@/components/entry-menu";
import { ErrorAlert } from "@/components/error-alert";
import { HeaderAction } from "@/components/header-action";
import { ImportMenu } from "@/components/import-menu";
import { LibraryList } from "@/components/library-list";
import { LIBRARY_RESULTS, LibraryToolbar } from "@/components/library-toolbar";
import { MangaCard } from "@/components/manga-card";
import { ScreenHeader } from "@/components/screen-header";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { useImport } from "@/hooks/use-import";
import { useLibraryView } from "@/hooks/use-library-view";
import { useScrollMemory } from "@/hooks/use-scroll-memory";
import { useShelf } from "@/hooks/use-shelf";
import { useT } from "@/i18n/use-t";
import { mangaKey } from "@/lib/format";
import type { Category } from "@/lib/ipc.library";
import { filterEntries, liveView, sortEntries } from "@/lib/library-view";
import { useNavigation } from "@/lib/navigation";
import { notifyError } from "@/lib/notify";
import { rememberLibraryQuery, savedLibraryQuery } from "@/lib/screen-memory";
import { useUpdates } from "@/lib/updates";

const noCategories: Category[] = [];

export function LibraryScreen() {
  const t = useT();
  const nav = useNavigation();
  const shelf = useShelf();
  const { state, reload } = shelf;
  const imports = useImport(reload);
  const [query, setQuery] = useState(savedLibraryQuery);
  const pane = useScrollMemory<HTMLDivElement>("library");
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
      shelf
        .unfollow(e)
        .catch((err: unknown) => notifyError(t("library.unfollowFailed", { title: e.title }), err)),
    onStatus: (e, status) =>
      shelf
        .setStatus(e, status)
        .catch((err: unknown) => notifyError(t("library.statusFailed"), err)),
    onToggleCategory: (e, id, on) =>
      shelf
        .toggleCategory(e, id, on)
        .catch((err: unknown) => notifyError(t("library.categoriesFailed"), err)),
    onEditCategories: () => setEditing(true),
  };
  const count = state.kind === "ready" ? entries.length : null;
  const filtered = query.trim() !== "" || view.category !== null || !!view.source || !!view.status;

  return (
    <section className="flex h-full flex-col">
      <ScreenHeader
        title={t("library.title")}
        subtitle={count !== null && t("library.followed", { count })}
      >
        {count ? (
          <div className="relative">
            <Search className="pointer-events-none absolute top-2 left-2 size-4 text-ink-3" />
            <Input
              type="search"
              value={query}
              onChange={(e) => {
                setQuery(e.target.value);
                rememberLibraryQuery(e.target.value);
              }}
              placeholder={t("library.filterByTitle")}
              aria-label={t("library.filterByTitle")}
              data-shortcut-search
              className="w-56 pl-7"
            />
          </div>
        ) : null}
        {count ? <CheckUpdatesButton /> : null}
        <ImportMenu busy={imports.busy} onPick={imports.start} />
        <HeaderAction
          variant="primary"
          keepLabel
          icon={<Plus />}
          label={t("library.follow")}
          onClick={() => nav.go({ screen: "search" })}
        />
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
      <div
        ref={pane}
        id={LIBRARY_RESULTS}
        className="relative min-h-0 flex-1 overflow-auto overscroll-contain"
      >
        {state.kind === "loading" && <p className="p-4 text-ink-3">{t("library.loading")}</p>}
        {state.kind === "error" && (
          <div className="p-4">
            <ErrorAlert title={t("library.openFailed")} message={state.message} onRetry={reload} />
          </div>
        )}
        {state.kind === "ready" && entries.length === 0 && (
          <EmptyState
            icon={<Library />}
            title={t("library.emptyTitle")}
            action={
              <Button variant="primary" onClick={() => nav.go({ screen: "search" })}>
                <Search /> {t("library.searchSource")}
              </Button>
            }
          >
            {t("library.emptyBody")}
          </EmptyState>
        )}
        {entries.length > 0 && items.length === 0 && filtered && (
          <EmptyState title={t("library.noMatchTitle")}>
            {query.trim()
              ? t("library.noMatchQuery", { query: query.trim() })
              : t("library.noMatchFilters")}
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
