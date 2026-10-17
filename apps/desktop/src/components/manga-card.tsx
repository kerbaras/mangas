import { ExternalLink, Trash2 } from "lucide-react";
import { Cover } from "@/components/cover";
import { Badge } from "@/components/ui/badge";
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuTrigger,
} from "@/components/ui/context-menu";
import { statusLabel } from "@/lib/format";
import { isLocal, type Manga } from "@/lib/ipc";

type Props = { manga: Manga; onOpen: (m: Manga) => void; onUnfollow: (m: Manga) => void };

export function MangaCard({ manga, onOpen, onUnfollow }: Props) {
  return (
    <ContextMenu>
      <ContextMenuTrigger asChild>
        <button
          type="button"
          onClick={() => onOpen(manga)}
          className="group w-full rounded-md p-1.5 text-left outline-none transition-colors hover:bg-surface-2 focus-visible:ring-2 focus-visible:ring-accent"
        >
          <Cover manga={manga} className="transition-transform group-hover:-translate-y-0.5" />
          <p className="mt-2 truncate text-body font-medium" title={manga.title}>
            {manga.title}
          </p>
          <div className="flex items-center gap-1.5 text-caption text-ink-3">
            {isLocal(manga) && <Badge variant="outline">Local</Badge>}
            {!isLocal(manga) && <span className="truncate">{statusLabel[manga.status]}</span>}
          </div>
        </button>
      </ContextMenuTrigger>
      <ContextMenuContent>
        <ContextMenuItem onSelect={() => onOpen(manga)}>
          <ExternalLink /> Open
        </ContextMenuItem>
        <ContextMenuSeparator />
        <ContextMenuItem onSelect={() => onUnfollow(manga)}>
          <Trash2 /> {isLocal(manga) ? "Remove" : "Unfollow"}
        </ContextMenuItem>
      </ContextMenuContent>
    </ContextMenu>
  );
}
