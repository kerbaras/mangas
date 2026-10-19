import { AppUpdateSettings } from "@/components/app-update-settings";
import { DiagnosticsPanel } from "@/components/diagnostics-panel";
import { SettingsGroup } from "@/components/settings-group";

/** Settings → About: version, app updates and diagnostics. */
export function AboutSettings() {
  return (
    <SettingsGroup
      title="About"
      resets={["appUpdateChannel", "appUpdateChecks"]}
      resetHint="Updates come from the stable channel again, and the app checks at launch."
    >
      <AppUpdateSettings />
      <DiagnosticsPanel />
    </SettingsGroup>
  );
}
