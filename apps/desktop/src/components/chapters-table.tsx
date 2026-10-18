import { ChapterRow } from "@/components/chapter-row";
import { Checkbox } from "@/components/ui/checkbox";
import type { DownloadMap } from "@/hooks/use-downloads";
import { isUnavailable, type Selection } from "@/lib/chapters";
import type { Chapter } from "@/lib/ipc";

type Props = {
  chapters: Chapter[];
  selected: Selection;
  downloads: DownloadMap;
  onToggle: (id: string, shift: boolean) => void;
  onToggleAll: () => void;
};

export function ChaptersTable({ chapters, selected, downloads, onToggle, onToggleAll }: Props) {
  const selectable = chapters.filter((c) => !isUnavailable(c));
  const allOn = selectable.length > 0 && selectable.every((c) => selected.has(c.id));
  const someOn = !allOn && selectable.some((c) => selected.has(c.id));
  return (
    <table className="w-full table-fixed border-collapse">
      <thead className="sticky top-0 z-10 bg-canvas text-left text-caption font-medium text-ink-3">
        <tr className="h-8 border-b border-stroke">
          <th className="w-10 pl-4">
            <Checkbox
              checked={allOn}
              ref={(el) => {
                if (el) el.indeterminate = someOn;
              }}
              disabled={selectable.length === 0}
              aria-label="Select all chapters"
              onChange={onToggleAll}
            />
          </th>
          <th className="w-16">Vol.</th>
          <th className="w-16">Ch.</th>
          <th>Title</th>
          <th className="w-40">Group</th>
          <th className="w-28">Language</th>
          <th className="w-44">State</th>
        </tr>
      </thead>
      <tbody>
        {chapters.map((c) => (
          <ChapterRow
            key={c.id}
            chapter={c}
            selected={selected.has(c.id)}
            progress={downloads.get(c.id)}
            onToggle={onToggle}
          />
        ))}
      </tbody>
    </table>
  );
}
