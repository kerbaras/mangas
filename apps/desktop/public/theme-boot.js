// Sets the palette before the first paint from the theme the app last applied (`lib/theme.ts`
// keeps it in localStorage), so a cold start never flashes the other one. Nothing kept yet
// (first launch) means the OS decides.
(() => {
  let theme = null;
  try {
    theme = window.localStorage.getItem("mangas.theme");
  } catch (_) {
    theme = null;
  }
  const media = window.matchMedia?.("(prefers-color-scheme: dark)");
  const dark = theme === "dark" || (theme !== "light" && Boolean(media?.matches));
  const root = document.documentElement;
  root.classList.toggle("dark", dark);
  root.classList.toggle("light", !dark);
  root.style.colorScheme = dark ? "dark" : "light";
})();
