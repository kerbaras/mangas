import { render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { setBackend } from "@/lib/ipc";
import { mockBackend } from "@/lib/ipc.mock";
import { loadSettings } from "@/lib/settings";
import { SettingsScreen } from "@/screens/settings";

afterEach(() => setBackend(null));

// An absolute element whose nearest positioned ancestor is outside the pane escapes its
// overflow and makes the whole document scrollable (macOS then rubber-bands the window).
describe("settings scroll pane", () => {
  it("owns the scroll, contains overscroll and positions every sr-only control", async () => {
    setBackend(mockBackend);
    await loadSettings();
    render(<SettingsScreen />);
    await screen.findByText("Cut preset");
    const pane = document.querySelector(".overflow-auto");
    expect(pane).toHaveClass("relative", "min-h-0", "flex-1", "overscroll-contain");
    const hidden = [...document.querySelectorAll(".sr-only")];
    expect(hidden.length).toBeGreaterThan(0);
    for (const el of hidden) expect(pane?.contains(el)).toBe(true);
  });
});
