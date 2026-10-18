import { Cover } from "@/components/cover";
import { type EntryHandlers, EntryMenu } from "@/components/entry-menu";
import { Badge } from "@/components/ui/badge";
import { ContextMenu, ContextMenuTrigger } from "@/components/ui/context-menu";
import { statusLabel } from "@/lib/format";
import { isLocal } from "@/lib/ipc";
import type { Category, LibraryEntry } from "@/lib/ipc.library";

type Props = EntryHandlers & { entry: LibraryEntry; categories: Category[] };

export function MangaCard({ entry, categories, ...on }: Props) {
  const unread = entry.chapters.unread;
  return (
    <ContextMenu>
      <ContextMenuTrigger asChild>
        <button
          type="button"
          onClick={() => on.onOpen(entry)}
          className="group w-full rounded-md p-1.5 text-left outline-none transition-colors hover:bg-surface-2 focus-visible:ring-2 focus-visible:ring-accent"
        >
          <div className="relative transition-transform group-hover:-translate-y-0.5">
            <Cover manga={entry} />
            {unread > 0 && (
              <span
                title={`${unread} unread`}
                className="absolute top-1.5 right-1.5 min-w-5 rounded-sm bg-accent px-1 text-center text-caption font-semibold text-white tabular-nums shadow-sm"
              >
                {unread}
              </span>
            )}
          </div>
          <p className="mt-2 truncate text-body font-medium" title={entry.title}>
            {entry.title}
          </p>
          <div className="flex items-center gap-1.5 text-caption text-ink-3">
            {isLocal(entry) && <Badge variant="outline">Local</Badge>}
            {!isLocal(entry) && <span className="truncate">{statusLabel[entry.status]}</span>}
          </div>
        </button>
      </ContextMenuTrigger>
      <EntryMenu entry={entry} categories={categories} {...on} />
    </ContextMenu>
  );
}
