import { Cover } from "@/components/cover";
import { type EntryHandlers, EntryMenu } from "@/components/entry-menu";
import { ContextMenu, ContextMenuTrigger } from "@/components/ui/context-menu";
import { mangaKey } from "@/lib/format";
import { type Category, type LibraryEntry, readingLabel } from "@/lib/ipc.library";

type Props = EntryHandlers & { entries: LibraryEntry[]; categories: Category[] };

const shortDate = (iso: string) =>
  new Date(iso).toLocaleDateString(undefined, { day: "numeric", month: "short", year: "numeric" });

/** Dense view: one row per title with its chapter state, same menu as the cards. */
export function LibraryList({ entries, categories, ...on }: Props) {
  return (
    <table className="w-full table-fixed border-collapse">
      <thead className="sticky top-0 z-10 bg-canvas text-left text-caption font-medium text-ink-3">
        <tr className="h-8 border-b border-stroke">
          <th className="w-14 pl-4" aria-label="Cover" />
          <th>Title</th>
          <th className="w-28">Source</th>
          <th className="w-28">Status</th>
          <th className="w-20 text-right">Chapters</th>
          <th className="w-20 text-right">On disk</th>
          <th className="w-20 text-right">Unread</th>
          <th className="w-36 pr-4 pl-6">Updated</th>
        </tr>
      </thead>
      <tbody>
        {entries.map((e) => (
          <ContextMenu key={mangaKey(e)}>
            <ContextMenuTrigger asChild>
              <tr
                onClick={() => on.onOpen(e)}
                className="h-12 cursor-default border-b border-stroke/60 text-body transition-colors hover:bg-surface-2/60"
              >
                <td className="pl-4">
                  <Cover manga={e} className="w-7" />
                </td>
                <td className="truncate pr-4 font-medium" title={e.title}>
                  {e.title}
                </td>
                <td className="truncate text-ink-2">{e.source}</td>
                <td className="truncate text-ink-2">{readingLabel(e.reading_status)}</td>
                <td className="text-right text-ink-2 tabular-nums">{e.chapters.total}</td>
                <td className="text-right text-ink-2 tabular-nums">{e.chapters.downloaded}</td>
                <td className="text-right tabular-nums">
                  {e.chapters.unread > 0 ? (
                    <span className="font-semibold text-accent">{e.chapters.unread}</span>
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
