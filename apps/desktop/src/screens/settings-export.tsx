import { DeviceSelect } from "@/components/device-select";
import { DevicesPending } from "@/components/devices-pending";
import { Field } from "@/components/field";
import { FormatPicker } from "@/components/format-picker";
import { PresetPicker } from "@/components/preset-picker";
import { SettingsGroup } from "@/components/settings-group";
import { useAsync } from "@/hooks/use-async";
import { effectiveFormat } from "@/lib/export-formats";
import { listDevices } from "@/lib/ipc";
import { useSettings } from "@/lib/settings";

/** Settings → Export: what the export screen starts with. */
export function ExportSettings() {
  const [settings, update] = useSettings();
  const devices = useAsync(listDevices, []);
  const list = devices.state.kind === "ready" ? devices.state.data : [];
  return (
    <SettingsGroup
      title="Export"
      resets={["defaultDevice", "defaultFormat", "defaultPreset"]}
      resetHint="The default device, format and cut preset go back to Kindle Paperwhite, the device's format and Balanced."
    >
      <Field label="Default device" htmlFor="default-device" hint="Preselected when exporting.">
        {devices.state.kind === "ready" ? (
          <DeviceSelect
            id="default-device"
            devices={list}
            value={settings.defaultDevice}
            onChange={(id) => update({ defaultDevice: id })}
          />
        ) : (
          <DevicesPending state={devices.state} />
        )}
      </Field>
      <Field
        label="Default format"
        hint={
          settings.defaultFormat ? (
            <>
              Used for every device.{" "}
              <button
                type="button"
                className="text-accent hover:underline"
                onClick={() => update({ defaultFormat: null })}
              >
                Follow the device instead
              </button>
            </>
          ) : (
            "Follows the device until you pick one."
          )
        }
      >
        <FormatPicker
          compact
          value={effectiveFormat(settings.defaultFormat, list, settings.defaultDevice)}
          onChange={(f) => update({ defaultFormat: f })}
        />
      </Field>
      <Field label="Default cut preset">
        <PresetPicker
          compact
          value={settings.defaultPreset}
          onChange={(p) => update({ defaultPreset: p })}
        />
      </Field>
    </SettingsGroup>
  );
}
