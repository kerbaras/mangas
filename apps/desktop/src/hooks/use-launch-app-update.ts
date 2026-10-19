import { useEffect, useRef } from "react";
import { checkForAppUpdate } from "@/lib/app-update";
import { useSettings, useSettingsLoad } from "@/lib/settings";

/** One silent app update check per launch, once settings are in, unless checks are off. */
export function useLaunchAppUpdate() {
  const ready = useSettingsLoad().kind === "ready";
  const [{ appUpdateChecks, appUpdateChannel }] = useSettings();
  const launched = useRef(false);

  useEffect(() => {
    if (!ready || launched.current) return;
    launched.current = true;
    if (appUpdateChecks) void checkForAppUpdate(appUpdateChannel, { silent: true });
  }, [ready, appUpdateChecks, appUpdateChannel]);
}
