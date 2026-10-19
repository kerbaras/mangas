import { describe, expect, it } from "vitest";
import { nextIndex } from "@/lib/list-nav";
import { actionFor, keyLabel, SHORTCUTS } from "@/lib/shortcuts";

const press = (key: string, mods: { meta?: boolean; ctrl?: boolean; alt?: boolean } = {}) => ({
  key,
  metaKey: !!mods.meta,
  ctrlKey: !!mods.ctrl,
  altKey: !!mods.alt,
});

describe("shortcut table", () => {
  it("uses Cmd on macOS and Ctrl elsewhere", () => {
    expect(actionFor(press("k", { meta: true }), true)).toEqual({ kind: "focus-search" });
    expect(actionFor(press("k", { ctrl: true }), false)).toEqual({ kind: "focus-search" });
    expect(actionFor(press("k", { ctrl: true }), true)).toBeNull();
    expect(actionFor(press("k", { meta: true }), false)).toBeNull();
    expect(actionFor(press("K", { ctrl: true }), false)).toEqual({ kind: "focus-search" });
    expect(actionFor(press("k", { ctrl: true, alt: true }), false)).toBeNull();
  });

  it("covers every shortcut the issue lists", () => {
    const on = (key: string, mod = false) => actionFor(press(key, { ctrl: mod }), false);
    expect(on(",", true)).toEqual({ kind: "go", route: { screen: "settings" } });
    expect(["1", "2", "3", "4"].map((n) => on(n, true))).toEqual(
      ["library", "search", "downloads", "export"].map((screen) => ({
        kind: "go",
        route: { screen },
      })),
    );
    expect(on("r", true)).toEqual({ kind: "refresh" });
    expect(on("Escape")).toEqual({ kind: "back" });
    expect(on("?")).toEqual({ kind: "help" });
    expect(on("ArrowUp")).toEqual({ kind: "list", move: "up" });
    expect(on("ArrowDown")).toEqual({ kind: "list", move: "down" });
    expect(on("ArrowLeft")).toEqual({ kind: "list", move: "left" });
    expect(on("ArrowRight")).toEqual({ kind: "list", move: "right" });
    expect(on("Enter")).toEqual({ kind: "list", move: "open" });
    expect(on("k")).toBeNull();
    expect(on("Escape", true)).toBeNull();
  });

  it("labels keys per platform", () => {
    expect(keyLabel({ key: "k", mod: true }, true)).toEqual(["⌘", "K"]);
    expect(keyLabel({ key: "k", mod: true }, false)).toEqual(["Ctrl", "K"]);
    expect(keyLabel({ key: "Escape" }, false)).toEqual(["Esc"]);
    expect(keyLabel({ key: "ArrowUp" }, false)).toEqual(["↑"]);
  });

  it("binds each key once", () => {
    const keys = SHORTCUTS.flatMap((s) => s.bindings.map((b) => `${b.mod ? "mod+" : ""}${b.key}`));
    expect(new Set(keys).size).toBe(keys.length);
  });
});

describe("list navigation", () => {
  it("walks a list and stops at the ends", () => {
    expect(nextIndex(-1, 3, 1, "down")).toBe(0);
    expect(nextIndex(-1, 3, 1, "up")).toBe(2);
    expect(nextIndex(0, 3, 1, "down")).toBe(1);
    expect(nextIndex(2, 3, 1, "down")).toBe(2);
    expect(nextIndex(0, 3, 1, "up")).toBe(0);
    expect(nextIndex(1, 3, 1, "right")).toBe(1);
    expect(nextIndex(-1, 0, 1, "down")).toBe(-1);
  });

  it("moves by rows in a grid", () => {
    expect(nextIndex(1, 10, 4, "down")).toBe(5);
    expect(nextIndex(5, 10, 4, "up")).toBe(1);
    expect(nextIndex(7, 10, 4, "down")).toBe(7);
    expect(nextIndex(5, 10, 4, "left")).toBe(4);
    expect(nextIndex(9, 10, 4, "right")).toBe(9);
    expect(nextIndex(4, 10, 4, "left")).toBe(4);
    expect(nextIndex(3, 10, 4, "right")).toBe(3);
  });
});
