import type { RefObject } from "react";
import { ChapterRow } from "@/components/chapter-row";
import { Checkbox } from "@/components/ui/checkbox";
import type { DownloadMap } from "@/hooks/use-downloads";
import { useRowWindow } from "@/hooks/use-row-window";
import { duplicateIds } from "@/lib/chapter-groups";
import { isUnavailable, type Selection } from "@/lib/chapters";
import type { Chapter } from "@/lib/ipc";

type Props = {
  chapters: Chapter[];
  selected: Selection;
  downloads: DownloadMap;
  onToggle: (id: string, shift: boolean) => void;
  onToggleAll: () => void;
  /** The pane that scrolls the table: only the rows in (and near) its viewport render. */
  scrollRef?: RefObject<HTMLElement | null>;
};

const COLUMNS = 7;

/** Stands in for the rows scrolled out of the pane, at their height. */
function Gap({ height }: { height: number }) {
  return (
    // biome-ignore lint/a11y/noAriaHiddenOnFocusable: an empty spacer row, never focusable
    <tr aria-hidden="true">
      <td colSpan={COLUMNS} className="p-0" style={{ height }} />
    </tr>
  );
}

export function ChaptersTable(props: Props) {
  const { chapters, selected, downloads, onToggle, onToggleAll, scrollRef } = props;
  const selectable = chapters.filter((c) => !isUnavailable(c));
  const allOn = selectable.length > 0 && selectable.every((c) => selected.has(c.id));
  const someOn = !allOn && selectable.some((c) => selected.has(c.id));
  const twins = duplicateIds(chapters);
  const rows = useRowWindow(chapters.length, scrollRef);
  return (
    <table className="w-full table-fixed border-collapse" aria-rowcount={chapters.length + 1}>
      <thead className="sticky top-0 z-10 bg-canvas text-left text-caption font-medium text-ink-3">
        <tr className="h-8 border-b border-stroke" aria-rowindex={1}>
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
      <tbody onFocus={rows.onFocus} onBlur={rows.onBlur}>
        {rows.slots.map((slot) => {
          if (slot.kind === "gap") return <Gap key={`gap-${slot.before}`} height={slot.height} />;
          const c = chapters[slot.index];
          if (!c) return null;
          return (
            <ChapterRow
              key={c.id}
              index={slot.index}
              measure={rows.measure}
              chapter={c}
              selected={selected.has(c.id)}
              showSourceId={twins.has(c.id)}
              progress={downloads.get(c.id)}
              onToggle={onToggle}
            />
          );
        })}
      </tbody>
    </table>
  );
}
