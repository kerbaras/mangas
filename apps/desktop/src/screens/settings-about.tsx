import { DiagnosticsPanel } from "@/components/diagnostics-panel";
import { Field } from "@/components/field";
import { SettingsGroup } from "@/components/settings-group";
import { useAsync } from "@/hooks/use-async";
import { appVersion } from "@/lib/app-version";

/** Settings → About: version and diagnostics. The updater (#38) adds its section here. */
export function AboutSettings() {
  const version = useAsync(appVersion, []);
  return (
    <SettingsGroup title="About">
      <Field label="Version">
        <p className="text-ink-2 select-text">
          {version.state.kind === "ready" ? version.state.data : "…"}
        </p>
      </Field>
      <DiagnosticsPanel />
    </SettingsGroup>
  );
}
