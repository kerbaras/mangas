import { afterEach, describe, expect, it, vi } from "vitest";
import { applyTheme, isDark } from "@/lib/theme";

function fakeMedia(dark: boolean) {
  const listeners = new Set<() => void>();
  const media = {
    matches: dark,
    addEventListener: (_: string, l: () => void) => listeners.add(l),
    removeEventListener: (_: string, l: () => void) => listeners.delete(l),
  };
  vi.stubGlobal("matchMedia", () => media);
  return {
    listeners,
    flip(next: boolean) {
      media.matches = next;
      for (const l of listeners) l();
    },
  };
}

afterEach(() => vi.unstubAllGlobals());

describe("theme", () => {
  it("picks dark for dark, light for light, the OS for system", () => {
    expect(isDark("dark", false)).toBe(true);
    expect(isDark("light", true)).toBe(false);
    expect(isDark("system", true)).toBe(true);
    expect(isDark("system", false)).toBe(false);
  });

  it("system follows the OS while applied, and stops listening when replaced", () => {
    const os = fakeMedia(false);
    const root = document.createElement("html");
    const stop = applyTheme("system", root);
    expect(root.className).toBe("light");
    os.flip(true);
    expect(root.className).toBe("dark");
    stop();
    expect(os.listeners.size).toBe(0);
    applyTheme("light", root);
    expect(root.className).toBe("light");
    expect(root.style.colorScheme).toBe("light");
  });
});
