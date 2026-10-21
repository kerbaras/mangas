import { defaultRangeExtractor, type Range, useVirtualizer } from "@tanstack/react-virtual";
import { type FocusEvent, type RefObject, useCallback, useRef } from "react";

/** `h-9` rows; the real height is measured once a row mounts. */
const ROW_HEIGHT = 36;
/** Rows kept mounted past each edge of the pane, so arrow keys always find the next row. */
const OVERSCAN = 12;

/** A row to render, or the empty band that stands in for rows that are scrolled away. */
export type WindowSlot =
  | { kind: "row"; index: number }
  | { kind: "gap"; height: number; before: number };

export type RowWindow = {
  slots: WindowSlot[];
  /** Row ref: reports the row's real height (rows need `data-index`). */
  measure?: (el: HTMLElement | null) => void;
  /** On the rows' parent: keeps the focused row mounted while it's scrolled out of the pane. */
  onFocus?: (e: FocusEvent<HTMLElement>) => void;
  onBlur?: (e: FocusEvent<HTMLElement>) => void;
};

const allRows = (count: number): WindowSlot[] =>
  Array.from({ length: count }, (_, index) => ({ kind: "row", index }));

const measuredHeight = (el: Element) => el.getBoundingClientRect().height || ROW_HEIGHT;

const indexOf = (el: EventTarget | null) => {
  const row = el instanceof Element ? el.closest<HTMLElement>("[data-index]") : null;
  return row ? Number(row.dataset.index) : null;
};

/**
 * Only the rows near `scrollRef`'s viewport, with gaps of the right height for the rest, so a
 * long list costs a screenful of rows to render and lay out. Without a pane every row renders.
 */
export function useRowWindow(count: number, scrollRef?: RefObject<HTMLElement | null>): RowWindow {
  const focused = useRef<number | null>(null);
  const rangeExtractor = useCallback((range: Range) => {
    const indexes = defaultRangeExtractor(range);
    const f = focused.current;
    if (f === null) return indexes;
    // The rows beside the focused one too, so ↑/↓ reach them rather than the far window.
    const kept = [f - 1, f, f + 1].filter((i) => i >= 0 && i < range.count && !indexes.includes(i));
    return kept.length === 0 ? indexes : [...indexes, ...kept].sort((a, b) => a - b);
  }, []);
  const windowed = scrollRef !== undefined;
  const v = useVirtualizer({
    count,
    enabled: windowed,
    getScrollElement: () => scrollRef?.current ?? null,
    estimateSize: () => ROW_HEIGHT,
    measureElement: measuredHeight,
    overscan: OVERSCAN,
    rangeExtractor,
    initialRect: { width: 0, height: typeof window === "undefined" ? 0 : window.innerHeight },
  });
  if (!windowed) return { slots: allRows(count) };
  const slots: WindowSlot[] = [];
  let end = 0;
  for (const item of v.getVirtualItems()) {
    if (item.start > end) slots.push({ kind: "gap", height: item.start - end, before: item.index });
    slots.push({ kind: "row", index: item.index });
    end = item.end;
  }
  const rest = v.getTotalSize() - end;
  if (rest > 0) slots.push({ kind: "gap", height: rest, before: count });
  return {
    slots,
    measure: v.measureElement,
    onFocus: (e) => {
      focused.current = indexOf(e.target);
    },
    onBlur: (e) => {
      if (!e.currentTarget.contains(e.relatedTarget as Node | null)) focused.current = null;
    },
  };
}
