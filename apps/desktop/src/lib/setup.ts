import { useSyncExternalStore } from "react";
import { useSettings, useSettingsLoad } from "@/lib/settings";

/** Set by Settings → "Run setup again"; first-run setup comes from the stored flag. */
let reopened = false;
const listeners = new Set<() => void>();
const notify = () => {
  for (const l of listeners) l();
};
const subscribe = (l: () => void) => {
  listeners.add(l);
  return () => listeners.delete(l);
};
const read = () => reopened;

export function requestSetup() {
  reopened = true;
  notify();
}

export function closeSetup() {
  reopened = false;
  notify();
}

/** Whether the setup sheet shows: a fresh install until it's finished or skipped, or on request. */
export const needsSetup = (setupComplete: boolean | null) => setupComplete === false;

/** Whether the sheet was reopened from Settings (not shown for a pending first run). */
export const useSetupReopened = (): boolean => useSyncExternalStore(subscribe, read, read);

export function useSetupOpen(): boolean {
  const [settings] = useSettings();
  const load = useSettingsLoad();
  const again = useSyncExternalStore(subscribe, read, read);
  return again || (load.kind === "ready" && needsSetup(settings.setupComplete));
}
