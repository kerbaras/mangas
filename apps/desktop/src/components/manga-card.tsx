import { Cover } from "@/components/cover";
import { type EntryHandlers, EntryMenu } from "@/components/entry-menu";
import { Hint } from "@/components/hint";
import { Badge } from "@/components/ui/badge";
import { ContextMenu, ContextMenuTrigger } from "@/components/ui/context-menu";
import { useT } from "@/i18n/use-t";
import { statusLabel } from "@/lib/format";
import { isLocal } from "@/lib/ipc";
import type { Category, LibraryEntry } from "@/lib/ipc.library";

type Props = EntryHandlers & { entry: LibraryEntry; categories: Category[] };

export function MangaCard({ entry, categories, ...on }: Props) {
  const t = useT();
  const unread = entry.chapters.unread;
  const fresh = entry.chapters.new;
  return (
    <ContextMenu>
      <ContextMenuTrigger asChild>
        <button
          type="button"
          data-nav-item
          onClick={() => on.onOpen(entry)}
          className="group w-full rounded-md p-1.5 text-left outline-none transition-colors hover:bg-surface-2 focus-visible:ring-2 focus-visible:ring-accent"
        >
          <div className="relative transition-transform group-hover:-translate-y-0.5">
            <Cover manga={entry} clearTop={fresh > 0 || unread > 0} />
            {fresh > 0 && (
              <Hint hint={t("library.newSince", { count: fresh })} focusable={false}>
                <span className="absolute top-1.5 left-1.5 rounded-sm bg-ok px-1 text-caption font-semibold text-canvas tabular-nums shadow-sm">
                  {t("library.newCount", { count: fresh })}
                </span>
              </Hint>
            )}
            {unread > 0 && (
              <Hint hint={t("library.unread", { count: unread })} focusable={false}>
                <span className="absolute top-1.5 right-1.5 min-w-5 rounded-sm bg-accent-text px-1 text-center text-caption font-semibold text-canvas tabular-nums shadow-sm">
                  {unread}
                </span>
              </Hint>
            )}
          </div>
          <Hint hint={entry.title} focusable={false} truncated>
            <p className="mt-2 truncate text-body font-medium">{entry.title}</p>
          </Hint>
          <div className="flex items-center gap-1.5 text-caption text-ink-3">
            {isLocal(entry) && <Badge variant="outline">{t("library.local")}</Badge>}
            {!isLocal(entry) && entry.status !== "unknown" && (
              <span className="truncate">{statusLabel[entry.status]}</span>
            )}
          </div>
        </button>
      </ContextMenuTrigger>
      <EntryMenu entry={entry} categories={categories} {...on} />
    </ContextMenu>
  );
}
