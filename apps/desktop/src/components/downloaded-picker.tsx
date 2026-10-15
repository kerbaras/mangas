import { Checkbox } from "@/components/ui/checkbox";
import type { DownloadedGroup } from "@/hooks/use-downloaded-chapters";
import { chapterLabel, type Selection, toggle, toggleAll } from "@/lib/chapters";

type Props = { groups: DownloadedGroup[]; selected: Selection; onChange: (s: Set<string>) => void };

/** Downloaded chapters grouped under their manga; the group checkbox selects the whole run. */
export function DownloadedPicker({ groups, selected, onChange }: Props) {
  return (
    <div className="flex flex-col gap-4">
      {groups.map(({ manga, chapters }) => {
        const ids = chapters.map((c) => c.id);
        const on = ids.filter((id) => selected.has(id)).length;
        return (
          <section key={`${manga.source}:${manga.id}`}>
            <header className="flex h-8 items-center gap-2">
              <Checkbox
                checked={on === ids.length}
                ref={(el) => {
                  if (el) el.indeterminate = on > 0 && on < ids.length;
                }}
                aria-label={`Select all of ${manga.title}`}
                onChange={() => onChange(toggleAll(selected, ids))}
              />
              <h2 className="truncate text-body font-semibold">{manga.title}</h2>
              <span className="text-caption text-ink-3 tabular-nums">
                {on}/{ids.length}
              </span>
            </header>
            <ul className="ml-6">
              {chapters.map((c) => (
                <li key={c.id}>
                  <label
                    htmlFor={`pick-${c.id}`}
                    className="flex h-7 cursor-default items-center gap-2 rounded-sm px-1 text-body hover:bg-surface-2"
                  >
                    <Checkbox
                      id={`pick-${c.id}`}
                      checked={selected.has(c.id)}
                      onChange={() => onChange(toggle(selected, c.id))}
                    />
                    <span className="truncate">{chapterLabel(c)}</span>
                  </label>
                </li>
              ))}
            </ul>
          </section>
        );
      })}
    </div>
  );
}
