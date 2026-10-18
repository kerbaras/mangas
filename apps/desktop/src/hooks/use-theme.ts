import { useEffect } from "react";
import { useSettings } from "@/lib/settings";
import { applyTheme } from "@/lib/theme";

/** Keeps the window's palette on the theme setting, switching instantly when it changes. */
export function useTheme() {
  const [{ theme }] = useSettings();
  useEffect(() => applyTheme(theme), [theme]);
}
