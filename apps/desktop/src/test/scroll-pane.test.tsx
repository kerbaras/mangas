import { fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { setBackend } from "@/lib/ipc";
import { mockBackend } from "@/lib/ipc.mock";
import { loadSettings } from "@/lib/settings";
import { SettingsScreen } from "@/screens/settings";
import { GROUPS } from "@/screens/settings-groups";

afterEach(() => setBackend(null));

// An absolute element whose nearest positioned ancestor is outside the pane escapes its
// overflow and makes the whole document scrollable (macOS then rubber-bands the window).
describe("settings scroll pane", () => {
  it("owns the scroll, contains overscroll and positions every sr-only control", async () => {
    setBackend(mockBackend);
    await loadSettings();
    render(<SettingsScreen />);
    const nav = await screen.findByRole("navigation", { name: "Settings groups" });
    const pane = nav.nextElementSibling;
    expect(pane).toHaveClass(
      "relative",
      "min-w-0",
      "flex-1",
      "overflow-y-auto",
      "overscroll-contain",
    );
    let hidden = 0;
    for (const g of GROUPS) {
      fireEvent.click(screen.getByRole("button", { name: g.label }));
      await screen.findByRole("button", { name: g.label, current: "page" });
      for (const el of document.querySelectorAll(".sr-only")) {
        expect(pane?.contains(el)).toBe(true);
        hidden += 1;
      }
    }
    expect(hidden).toBeGreaterThan(0);
  });
});
