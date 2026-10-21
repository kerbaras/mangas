import { ExternalLink, Tags, Trash2 } from "lucide-react";
import {
  ContextMenuCheckboxItem,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuLabel,
  ContextMenuRadioGroup,
  ContextMenuRadioItem,
  ContextMenuSeparator,
  ContextMenuSub,
  ContextMenuSubContent,
  ContextMenuSubTrigger,
} from "@/components/ui/context-menu";
import { useT } from "@/i18n/use-t";
import { isLocal } from "@/lib/ipc";
import {
  type Category,
  type LibraryEntry,
  READING_STATUSES,
  type ReadingStatus,
} from "@/lib/ipc.library";

/** What a library title's menu can do; the screen wires these to the shelf and notifies errors. */
export type EntryHandlers = {
  onOpen: (e: LibraryEntry) => void;
  onUnfollow: (e: LibraryEntry) => void;
  onStatus: (e: LibraryEntry, status: ReadingStatus) => void;
  onToggleCategory: (e: LibraryEntry, id: number, on: boolean) => void;
  onEditCategories: () => void;
};

type Props = EntryHandlers & { entry: LibraryEntry; categories: Category[] };

/** Right-click menu shared by the grid card and the list row. */
export function EntryMenu({ entry, categories, ...on }: Props) {
  const t = useT();
  return (
    <ContextMenuContent>
      <ContextMenuItem onSelect={() => on.onOpen(entry)}>
        <ExternalLink /> {t("library.menuOpen")}
      </ContextMenuItem>
      <ContextMenuSeparator />
      <ContextMenuSub>
        <ContextMenuSubTrigger>{t("library.readingStatus")}</ContextMenuSubTrigger>
        <ContextMenuSubContent>
          <ContextMenuRadioGroup
            value={entry.reading_status}
            onValueChange={(v) => on.onStatus(entry, v as ReadingStatus)}
          >
            {READING_STATUSES.map((s) => (
              <ContextMenuRadioItem key={s.id} value={s.id}>
                {s.label}
              </ContextMenuRadioItem>
            ))}
          </ContextMenuRadioGroup>
        </ContextMenuSubContent>
      </ContextMenuSub>
      <ContextMenuSub>
        <ContextMenuSubTrigger>{t("library.menuCategories")}</ContextMenuSubTrigger>
        <ContextMenuSubContent>
          {categories.length === 0 && (
            <ContextMenuLabel>{t("library.noCategories")}</ContextMenuLabel>
          )}
          {categories.map((c) => {
            const on_ = entry.categories.includes(c.id);
            return (
              <ContextMenuCheckboxItem
                key={c.id}
                checked={on_}
                onSelect={(e) => e.preventDefault()}
                onCheckedChange={(checked) => on.onToggleCategory(entry, c.id, checked === true)}
              >
                {c.name}
              </ContextMenuCheckboxItem>
            );
          })}
          <ContextMenuSeparator />
          <ContextMenuItem onSelect={on.onEditCategories}>
            <Tags /> {t("library.menuEditCategories")}
          </ContextMenuItem>
        </ContextMenuSubContent>
      </ContextMenuSub>
      <ContextMenuSeparator />
      <ContextMenuItem onSelect={() => on.onUnfollow(entry)}>
        <Trash2 /> {isLocal(entry) ? t("library.remove") : t("library.unfollow")}
      </ContextMenuItem>
    </ContextMenuContent>
  );
}
