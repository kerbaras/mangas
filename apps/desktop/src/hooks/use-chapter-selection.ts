import { useCallback, useState } from "react";
import { selectRange, toggle, toggleAll } from "@/lib/chapters";

/** Multi-select over the visible rows: click toggles, shift-click extends from the last click. */
export function useChapterSelection(visibleIds: string[]) {
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [anchor, setAnchor] = useState<string | null>(null);

  const onToggle = useCallback(
    (id: string, shift: boolean) => {
      setSelected((s) => (shift ? selectRange(s, visibleIds, anchor, id) : toggle(s, id)));
      setAnchor(id);
    },
    [visibleIds, anchor],
  );

  const onToggleAll = useCallback(() => setSelected((s) => toggleAll(s, visibleIds)), [visibleIds]);

  const clear = useCallback(() => setSelected(new Set()), []);
  return { selected, onToggle, onToggleAll, clear };
}
