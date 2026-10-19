import type { Route } from "@/lib/navigation";

/** What a shortcut does; `use-shortcuts` maps each to its handler. */
export type ShortcutAction =
  | { kind: "focus-search" }
  | { kind: "go"; route: Route }
  | { kind: "back" }
  | { kind: "refresh" }
  | { kind: "help" }
  | { kind: "list"; move: "up" | "down" | "left" | "right" | "open" };

/** `mod` is Cmd on macOS and Ctrl elsewhere. */
export type Binding = { key: string; mod?: boolean };

export type Shortcut = {
  bindings: Binding[];
  label: string;
  group: "General" | "Go to" | "Lists";
  action: ShortcutAction;
};

const go = (route: Route): ShortcutAction => ({ kind: "go", route });

/** The one table: the key handler and the `?` sheet both read it. */
export const SHORTCUTS: Shortcut[] = [
  {
    bindings: [{ key: "k", mod: true }],
    label: "Search",
    group: "General",
    action: { kind: "focus-search" },
  },
  {
    bindings: [{ key: "r", mod: true }],
    label: "Refresh this title",
    group: "General",
    action: { kind: "refresh" },
  },
  {
    bindings: [{ key: "Escape" }],
    label: "Back, or close a dialog",
    group: "General",
    action: { kind: "back" },
  },
  {
    bindings: [{ key: "?" }],
    label: "Keyboard shortcuts",
    group: "General",
    action: { kind: "help" },
  },
  {
    bindings: [{ key: "1", mod: true }],
    label: "Library",
    group: "Go to",
    action: go({ screen: "library" }),
  },
  {
    bindings: [{ key: "2", mod: true }],
    label: "Search",
    group: "Go to",
    action: go({ screen: "search" }),
  },
  {
    bindings: [{ key: "3", mod: true }],
    label: "Downloads",
    group: "Go to",
    action: go({ screen: "downloads" }),
  },
  {
    bindings: [{ key: "4", mod: true }],
    label: "Export",
    group: "Go to",
    action: go({ screen: "export" }),
  },
  {
    bindings: [{ key: ",", mod: true }],
    label: "Settings",
    group: "Go to",
    action: go({ screen: "settings" }),
  },
  {
    bindings: [{ key: "ArrowUp" }],
    label: "Previous item",
    group: "Lists",
    action: { kind: "list", move: "up" },
  },
  {
    bindings: [{ key: "ArrowDown" }],
    label: "Next item",
    group: "Lists",
    action: { kind: "list", move: "down" },
  },
  {
    bindings: [{ key: "ArrowLeft" }, { key: "ArrowRight" }],
    label: "Move across the library grid",
    group: "Lists",
    action: { kind: "list", move: "left" },
  },
  {
    bindings: [{ key: "Enter" }],
    label: "Open or select the item",
    group: "Lists",
    action: { kind: "list", move: "open" },
  },
];

export type KeyPress = Pick<KeyboardEvent, "key" | "metaKey" | "ctrlKey" | "altKey">;

const matches = (b: Binding, e: KeyPress, mac: boolean) => {
  const mod = mac ? e.metaKey : e.ctrlKey;
  const other = mac ? e.ctrlKey : e.metaKey;
  return !e.altKey && !other && mod === !!b.mod && e.key.toLowerCase() === b.key.toLowerCase();
};

/** The action a key press triggers; `ArrowRight` is the grid move's other direction. */
export function actionFor(e: KeyPress, mac: boolean): ShortcutAction | null {
  for (const s of SHORTCUTS) {
    const hit = s.bindings.find((b) => matches(b, e, mac));
    if (!hit) continue;
    if (s.action.kind === "list" && hit.key === "ArrowRight")
      return { kind: "list", move: "right" };
    return s.action;
  }
  return null;
}

const KEY_NAMES: Record<string, string> = {
  Escape: "Esc",
  ArrowUp: "↑",
  ArrowDown: "↓",
  ArrowLeft: "←",
  ArrowRight: "→",
  Enter: "Enter",
};

/** The keys of a binding as shown in the sheet, e.g. `["⌘", "K"]` or `["Ctrl", "K"]`. */
export function keyLabel(b: Binding, mac: boolean): string[] {
  const key = KEY_NAMES[b.key] ?? b.key.toUpperCase();
  return b.mod ? [mac ? "⌘" : "Ctrl", key] : [key];
}
