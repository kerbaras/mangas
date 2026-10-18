import { useEffect, useRef } from "react";
import { useSettings, useSettingsLoad } from "@/lib/settings";
import { intervalMs, runCheck } from "@/lib/updates";

const quietly = (background: boolean) =>
  runCheck(background).catch((e: unknown) => console.warn("update check failed", e));

/**
 * Background update checks: once at launch and every `updateCheckHours` after, never before
 * the stored settings are in (#99: providers and languages come from them). 0 hours = off.
 */
export function useUpdateSchedule() {
  const ready = useSettingsLoad().kind === "ready";
  const [{ updateCheckHours }] = useSettings();
  const launched = useRef(false);
  const every = ready ? intervalMs(updateCheckHours) : null;

  useEffect(() => {
    if (every === null || launched.current) return;
    launched.current = true;
    void quietly(true);
  }, [every]);

  useEffect(() => {
    if (every === null) return;
    const timer = setInterval(() => void quietly(true), every);
    return () => clearInterval(timer);
  }, [every]);
}
