import { useEffect } from "react";
import { useSettings, useSettingsLoad } from "@/lib/settings";
import { applyTheme, rememberedTheme, rememberTheme } from "@/lib/theme";

/**
 * Keeps the window's palette on the theme setting, switching instantly when it changes. Until the
 * stored settings are in, the theme kept from the last run stays on, not the placeholder default.
 */
export function useTheme() {
  const [{ theme }] = useSettings();
  const ready = useSettingsLoad().kind === "ready";
  const shown = ready ? theme : (rememberedTheme() ?? theme);
  useEffect(() => applyTheme(shown), [shown]);
  useEffect(() => {
    if (ready) rememberTheme(theme);
  }, [ready, theme]);
}
