/** Items `↑/↓/←/→ Enter` walk through: rows and cards marked `data-nav-item`. */
export const NAV_ITEM = "[data-nav-item]";

export type Move = "up" | "down" | "left" | "right";

/**
 * The index to focus after `move` from `current` (-1 when nothing is focused yet) in `count`
 * items laid out `columns` per row. Stays put at the edges.
 */
export function nextIndex(current: number, count: number, columns: number, move: Move): number {
  if (count === 0) return -1;
  if (current < 0) return move === "up" || move === "left" ? count - 1 : 0;
  const cols = Math.max(1, columns);
  const step = { up: -cols, down: cols, left: -1, right: 1 }[move];
  const column = current % cols;
  if ((move === "left" && column === 0) || (move === "right" && column === cols - 1))
    return current;
  const next = current + step;
  return next < 0 || next >= count ? current : next;
}

/** How many items share the first item's row (1 for a list). */
const columnsOf = (items: HTMLElement[]) =>
  items.filter((el) => el.offsetTop === items[0]?.offsetTop).length || 1;

/** Moves focus between the visible items under `root`; false when it has none. */
export function moveFocus(root: ParentNode, move: Move): boolean {
  const items = [...root.querySelectorAll<HTMLElement>(NAV_ITEM)].filter(
    (el) => el.getClientRects().length > 0,
  );
  if (items.length === 0) return false;
  const active = document.activeElement?.closest<HTMLElement>(NAV_ITEM) ?? null;
  const current = active ? items.indexOf(active) : -1;
  const target = items[nextIndex(current, items.length, columnsOf(items), move)];
  target?.focus();
  target?.scrollIntoView?.({ block: "nearest" });
  return true;
}

/** Enter on a focused item that isn't natively clickable by keyboard (a table row). */
export function openFocused(): boolean {
  const active = document.activeElement;
  if (!(active instanceof HTMLElement) || !active.matches(NAV_ITEM)) return false;
  if (active.matches("button, a, input, [role=switch]")) return false;
  active.click();
  return true;
}
