import { act, fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { ShortcutsSheet } from "@/components/shortcuts-sheet";
import { useShortcuts } from "@/hooks/use-shortcuts";
import type { Navigation, Route } from "@/lib/navigation";
import { useRefreshShortcut } from "@/lib/refresh-shortcut";
import { keyLabel, SHORTCUTS } from "@/lib/shortcuts";

function Harness({ nav, refresh }: { nav: Navigation; refresh?: () => void }) {
  const [help, setHelp] = useState(false);
  useShortcuts(nav, () => setHelp(true));
  useRefreshShortcut(refresh ?? (() => {}));
  return (
    <main>
      <input aria-label="filter" data-shortcut-search />
      <button type="button" data-nav-item>
        one
      </button>
      <button type="button" data-nav-item>
        two
      </button>
      <ShortcutsSheet open={help} onOpenChange={setHelp} />
    </main>
  );
}

const navAt = (route: Route, canGoBack = false) =>
  ({ route, canGoBack, go: vi.fn(), back: vi.fn() }) as Navigation & {
    go: ReturnType<typeof vi.fn>;
    back: ReturnType<typeof vi.fn>;
  };

const key = (k: string, ctrl = false, target: Element | Window = window) =>
  fireEvent.keyDown(target, { key: k, ctrlKey: ctrl });

beforeAll(() => {
  // jsdom lays nothing out; list navigation only walks visible items.
  HTMLElement.prototype.getClientRects = () => [{}] as unknown as DOMRectList;
  // One item per row, as in a list.
  Object.defineProperty(HTMLElement.prototype, "offsetTop", {
    get(this: HTMLElement) {
      return [...(this.parentElement?.children ?? [])].indexOf(this) * 10;
    },
  });
});

describe("global shortcuts", () => {
  it("navigates with Ctrl+1..4 and Ctrl+,", () => {
    const nav = navAt({ screen: "library" });
    render(<Harness nav={nav} />);
    for (const k of ["1", "2", "3", "4", ","]) key(k, true);
    expect(nav.go.mock.calls.map(([r]) => r.screen)).toEqual([
      "library",
      "search",
      "downloads",
      "export",
      "settings",
    ]);
  });

  it("focuses the screen's search field, or opens Search when it has none", () => {
    const nav = navAt({ screen: "library" });
    render(<Harness nav={nav} />);
    key("k", true);
    expect(document.activeElement).toBe(screen.getByLabelText("filter"));
    expect(nav.go).not.toHaveBeenCalled();
    screen.getByLabelText("filter").remove();
    key("k", true);
    expect(nav.go).toHaveBeenCalledWith({ screen: "search" });
  });

  it("goes back with Esc, blurs a field first, and leaves open dialogs to Radix", () => {
    const nav = navAt({ screen: "manga" } as Route, true);
    render(<Harness nav={nav} />);
    const field = screen.getByLabelText("filter");
    field.focus();
    key("Escape", false, field);
    expect(nav.back).not.toHaveBeenCalled();
    expect(document.activeElement).not.toBe(field);
    key("Escape");
    expect(nav.back).toHaveBeenCalledTimes(1);
    key("?");
    expect(screen.getByRole("dialog")).toBeTruthy();
    key("Escape", false, document.activeElement ?? window);
    expect(nav.back).toHaveBeenCalledTimes(1);
  });

  it("does nothing on Esc in the Library with nowhere to go back to", () => {
    const nav = navAt({ screen: "library" });
    render(<Harness nav={nav} />);
    key("Escape");
    expect(nav.back).not.toHaveBeenCalled();
  });

  it("refreshes the open title on Ctrl+R and never lets the webview reload", () => {
    const refresh = vi.fn();
    render(<Harness nav={navAt({ screen: "library" })} refresh={refresh} />);
    const event = new KeyboardEvent("keydown", { key: "r", ctrlKey: true, cancelable: true });
    act(() => void window.dispatchEvent(event));
    expect(refresh).toHaveBeenCalledTimes(1);
    expect(event.defaultPrevented).toBe(true);
  });

  it("moves through list items with the arrows, from the search field too", () => {
    render(<Harness nav={navAt({ screen: "search" })} />);
    const [one, two] = screen.getAllByRole("button");
    const field = screen.getByLabelText("filter");
    field.focus();
    key("ArrowDown", false, field);
    expect(document.activeElement).toBe(one);
    key("ArrowDown", false, one);
    expect(document.activeElement).toBe(two);
    key("ArrowUp", false, two);
    expect(document.activeElement).toBe(one);
  });

  it("ignores ? while typing", () => {
    render(<Harness nav={navAt({ screen: "library" })} />);
    const field = screen.getByLabelText("filter");
    field.focus();
    key("?", false, field);
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("shows every shortcut of the table in the ? sheet", () => {
    render(<Harness nav={navAt({ screen: "library" })} />);
    key("?");
    const sheet = screen.getByRole("dialog");
    for (const s of SHORTCUTS) {
      const group = sheet.querySelector(`section[aria-label="${s.group}"]`);
      const row = [...(group?.querySelectorAll("dt") ?? [])].find(
        (dt) => dt.textContent === s.label,
      );
      expect(row, s.label).toBeTruthy();
      const keys = [...(row?.nextElementSibling?.querySelectorAll("kbd") ?? [])].map(
        (k) => k.textContent,
      );
      expect(keys).toEqual(s.bindings.flatMap((b) => keyLabel(b, false)));
    }
  });
});
