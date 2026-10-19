import { afterEach, describe, expect, it, vi } from "vitest";
import { rememberedTheme, rememberTheme, THEME_KEY } from "@/lib/theme";
import boot from "../../public/theme-boot.js?raw";

const root = document.documentElement;

function coldStart(osDark: boolean) {
  vi.stubGlobal("matchMedia", () => ({ matches: osDark }));
  root.className = "";
  new Function(boot)();
  return root.classList.contains("dark") ? "dark" : "light";
}

afterEach(() => {
  window.localStorage.clear();
  vi.unstubAllGlobals();
});

describe("theme before first paint", () => {
  it("applies the kept theme over the OS one", () => {
    rememberTheme("dark");
    expect(coldStart(false)).toBe("dark");
    rememberTheme("light");
    expect(coldStart(true)).toBe("light");
  });

  it("follows the OS for system and on a first launch", () => {
    expect(coldStart(true)).toBe("dark");
    expect(coldStart(false)).toBe("light");
    rememberTheme("system");
    expect(coldStart(true)).toBe("dark");
  });

  it("reads back only a known theme", () => {
    expect(rememberedTheme()).toBeUndefined();
    rememberTheme("dark");
    expect(rememberedTheme()).toBe("dark");
    window.localStorage.setItem(THEME_KEY, "sepia");
    expect(rememberedTheme()).toBeUndefined();
  });
});
