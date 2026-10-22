import { Check, Plus } from "lucide-react";
import { Cover } from "@/components/cover";
import { StatusBadge } from "@/components/status-badge";
import { Button } from "@/components/ui/button";
import { useT } from "@/i18n/use-t";
import type { Manga } from "@/lib/ipc";

type Props = {
  manga: Manga;
  followed: boolean;
  busy: boolean;
  onFollow: (m: Manga) => void;
  onOpen: (m: Manga) => void;
};

export function SearchResult({ manga, followed, busy, onFollow, onOpen }: Props) {
  const t = useT();
  return (
    <li className="flex gap-4 rounded-md p-2 hover:bg-surface-2/60">
      <Cover manga={manga} className="w-14 shrink-0" />
      <div className="min-w-0 flex-1 py-0.5">
        <div className="flex items-center gap-2">
          <button
            type="button"
            data-nav-item
            onClick={() => onOpen(manga)}
            className="truncate text-left text-body font-medium hover:underline"
          >
            {manga.title}
          </button>
          <StatusBadge status={manga.status} />
        </div>
        <p className="mt-1 line-clamp-2 text-body text-ink-2">{manga.description}</p>
        <p className="mt-1 text-caption text-ink-3">{manga.source}</p>
      </div>
      <div className="shrink-0 self-center">
        {followed ? (
          <Button size="sm" variant="ghost" disabled>
            <Check /> {t("search.following")}
          </Button>
        ) : (
          <Button size="sm" variant="primary" disabled={busy} onClick={() => onFollow(manga)}>
            <Plus /> {t("search.follow")}
          </Button>
        )}
      </div>
    </li>
  );
}
