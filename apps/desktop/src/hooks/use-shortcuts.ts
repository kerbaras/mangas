import { useEffect, useRef } from "react";
import { moveFocus, openFocused } from "@/lib/list-nav";
import type { Navigation } from "@/lib/navigation";
import { isMacLike } from "@/lib/platform";
import { refreshCurrent } from "@/lib/refresh-shortcut";
import { actionFor, type ShortcutAction } from "@/lib/shortcuts";

/** The search field `⌘K` focuses on screens that have one. */
export const SEARCH_FIELD = "data-shortcut-search";

/** Open dialogs and menus handle their own keys (Radix closes them on Esc). */
const overlayOpen = () =>
  document.querySelector(
    '[role="dialog"][data-state="open"], [role="alertdialog"][data-state="open"], [role="menu"], [role="listbox"]',
  ) !== null;

const isTyping = (t: EventTarget | null): t is HTMLElement =>
  t instanceof HTMLElement && (t.isContentEditable || t.matches("input, textarea, select"));

/** Widgets whose arrow keys already mean something. */
const ownsArrows = (t: EventTarget | null) =>
  t instanceof HTMLElement &&
  t.closest("[role=tablist], [role=radiogroup], [role=slider]") !== null;

/** Runs `action`; true when it handled the key (so the webview shouldn't). */
function run(action: ShortcutAction, e: KeyboardEvent, nav: Navigation, help: () => void) {
  const typing = isTyping(e.target);
  const main = document.querySelector("main") ?? document;
  switch (action.kind) {
    case "focus-search": {
      const field = main.querySelector<HTMLInputElement>(`[${SEARCH_FIELD}]`);
      if (!field) nav.go({ screen: "search" });
      field?.focus();
      field?.select();
      return true;
    }
    case "go":
      nav.go(action.route);
      return true;
    case "back":
      if (typing) e.target.blur();
      else if (nav.canGoBack || nav.route.screen !== "library") nav.back();
      return true;
    case "refresh":
      refreshCurrent();
      return true;
    case "help":
      if (!typing) help();
      return !typing;
    case "list": {
      if (action.move === "open") return !typing && openFocused();
      const fromSearch = typing && action.move === "down" && e.target.hasAttribute(SEARCH_FIELD);
      if ((typing && !fromSearch) || ownsArrows(e.target)) return false;
      return moveFocus(main, action.move);
    }
  }
}

/** One window listener (capture, so it runs before Radix) for the whole `SHORTCUTS` table. */
export function useShortcuts(nav: Navigation, help: () => void) {
  const latest = useRef({ nav, help });
  latest.current = { nav, help };
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.defaultPrevented || e.isComposing) return;
      const action = actionFor(e, isMacLike());
      if (!action) return;
      // ⌘R would reload the webview even where there is no title to refresh.
      if (action.kind === "refresh") e.preventDefault();
      if (overlayOpen()) return;
      if (run(action, e, latest.current.nav, latest.current.help)) e.preventDefault();
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, []);
}
