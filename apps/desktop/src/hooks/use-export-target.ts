import { useAsync } from "@/hooks/use-async";
import { effectiveFormat } from "@/lib/export-formats";
import { type ExportFormat, listDevices } from "@/lib/ipc";
import { useSettings } from "@/lib/settings";

/**
 * Device profile, format and whether to replace existing files: the stored defaults, so picking
 * one keeps it for next time.
 */
export function useExportTarget() {
  const [settings, update] = useSettings();
  const devices = useAsync(listDevices, []);
  const list = devices.state.kind === "ready" ? devices.state.data : [];
  const device = settings.defaultDevice;
  return {
    devices: devices.state,
    device,
    format: effectiveFormat(settings.defaultFormat, list, device),
    recommended: list.find((d) => d.id === device),
    setDevice: (id: string) => update({ defaultDevice: id }),
    setFormat: (f: ExportFormat) => update({ defaultFormat: f }),
    replace: settings.replaceExisting,
    setReplace: (on: boolean) => update({ replaceExisting: on }),
  };
}

export type ExportTarget = ReturnType<typeof useExportTarget>;
