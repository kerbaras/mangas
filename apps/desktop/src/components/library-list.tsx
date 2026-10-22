import { Cover } from "@/components/cover";
import { type EntryHandlers, EntryMenu } from "@/components/entry-menu";
import { Hint } from "@/components/hint";
import { ContextMenu, ContextMenuTrigger } from "@/components/ui/context-menu";
import { useT } from "@/i18n/use-t";
import { mangaKey } from "@/lib/format";
import { type Category, type LibraryEntry, readingLabel } from "@/lib/ipc.library";

type Props = EntryHandlers & { entries: LibraryEntry[]; categories: Category[] };

const shortDate = (iso: string) =>
  new Date(iso).toLocaleDateString(undefined, { day: "numeric", month: "short", year: "numeric" });

/** Dense view: one row per title with its chapter state, same menu as the cards. */
export function LibraryList({ entries, categories, ...on }: Props) {
  const t = useT();
  return (
    <table className="w-full table-fixed border-collapse">
      <thead className="sticky top-0 z-10 bg-canvas text-left text-caption font-medium text-ink-3">
        <tr className="h-8 border-b border-stroke">
          <th className="w-14 pl-4">
            <span className="sr-only">{t("library.colCover")}</span>
          </th>
          <th>{t("library.colTitle")}</th>
          <th className="w-28">{t("library.colSource")}</th>
          <th className="w-28">{t("library.colStatus")}</th>
          <th className="w-20 text-right">{t("library.colChapters")}</th>
          <th className="w-20 text-right">{t("library.colOnDisk")}</th>
          <th className="w-20 text-right">{t("library.colUnread")}</th>
          <th className="w-36 pr-4 pl-6">{t("library.colUpdated")}</th>
        </tr>
      </thead>
      <tbody>
        {entries.map((e) => (
          <ContextMenu key={mangaKey(e)}>
            <ContextMenuTrigger asChild>
              <tr
                data-nav-item
                tabIndex={-1}
                onClick={() => on.onOpen(e)}
                className="h-12 cursor-default border-b border-stroke/60 text-body transition-colors hover:bg-surface-2/60 outline-none focus-visible:bg-surface-2 focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-accent"
              >
                <td className="pl-4">
                  <Cover manga={e} className="w-7" />
                </td>
                <Hint hint={e.title} focusable={false} truncated>
                  <td className="truncate pr-4 font-medium">
                    {e.chapters.new > 0 && (
                      <span className="mr-2 rounded-sm bg-ok px-1 text-caption font-semibold text-canvas tabular-nums">
                        {t("library.newCount", { count: e.chapters.new })}
                      </span>
                    )}
                    {e.title}
                  </td>
                </Hint>
                <td className="truncate text-ink-2">{e.source}</td>
                <td className="truncate text-ink-2">{readingLabel(e.reading_status)}</td>
                <td className="text-right text-ink-2 tabular-nums">{e.chapters.total}</td>
                <td className="text-right text-ink-2 tabular-nums">{e.chapters.downloaded}</td>
                <td className="text-right tabular-nums">
                  {e.chapters.unread > 0 ? (
                    <span className="font-semibold text-accent-text">{e.chapters.unread}</span>
                  ) : (
                    <span className="text-ink-3">—</span>
                  )}
                </td>
                <td className="truncate pr-4 pl-6 text-ink-2">{shortDate(e.updated_at)}</td>
              </tr>
            </ContextMenuTrigger>
            <EntryMenu entry={e} categories={categories} {...on} />
          </ContextMenu>
        ))}
      </tbody>
    </table>
  );
}
