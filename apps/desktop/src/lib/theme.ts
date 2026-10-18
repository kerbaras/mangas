/** Light, dark or the OS's: the `dark`/`light` class on <html> picks the token palette. */
import type { Theme } from "./ipc";

export const isDark = (theme: Theme, systemDark: boolean) =>
  theme === "dark" || (theme === "system" && systemDark);

/** Applies `theme` now and, for `system`, whenever the OS switches; returns the unsubscribe. */
export function applyTheme(theme: Theme, root: HTMLElement = document.documentElement) {
  const media = window.matchMedia?.("(prefers-color-scheme: dark)");
  const apply = () => {
    const dark = isDark(theme, media?.matches ?? false);
    root.classList.toggle("dark", dark);
    root.classList.toggle("light", !dark);
    root.style.colorScheme = dark ? "dark" : "light";
  };
  apply();
  if (theme !== "system") return () => {};
  media?.addEventListener("change", apply);
  return () => media?.removeEventListener("change", apply);
}
