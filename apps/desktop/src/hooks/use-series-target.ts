import { useState } from "react";
import { useExportTarget } from "@/hooks/use-export-target";
import { type Series, useSeriesPreset } from "@/hooks/use-series-preset";
import { effectiveFormat } from "@/lib/export-formats";
import type { ExportPick } from "@/lib/export-name";
import type { Preset } from "@/lib/ipc";
import { withQuality } from "@/lib/kindle-size";
import { useSettings } from "@/lib/settings";

/**
 * Device, format and cut preset of the export dialog: the stored defaults (picking a device or
 * format keeps it for next time) unless the one picked series has a preset that sets them.
 */
export function useSeriesTarget(picks: ExportPick[]) {
  const [settings, update] = useSettings();
  const stored = useExportTarget();
  const [ownPreset, setOwnPreset] = useState<Preset>(settings.defaultPreset);
  const [pick] = picks;
  const info: Series | null =
    picks.length === 1 && pick
      ? { source: pick.manga.source, id: pick.manga.id, title: pick.manga.title }
      : null;
  const series = useSeriesPreset(info);
  const list = stored.devices.kind === "ready" ? stored.devices.data : [];
  const device = series.shown("profile", stored.device);
  const target = {
    ...stored,
    device,
    format: series.shown("format", effectiveFormat(settings.defaultFormat, list, device)),
    recommended: list.find((d) => d.id === device),
    // The quality stored for the device shown, which a series preset may have picked.
    quality: settings.jpegQuality[device] ?? null,
    setQuality: (q: number | null) =>
      update({ jpegQuality: withQuality(settings.jpegQuality, device, q) }),
    setDevice: (id: string) => {
      series.touch("profile");
      stored.setDevice(id);
    },
    setFormat: (f: Parameters<typeof stored.setFormat>[0]) => {
      series.touch("format");
      stored.setFormat(f);
    },
  };
  const setPreset = (p: Preset) => {
    series.touch("cut_preset");
    setOwnPreset(p);
  };
  const cut = { series: info, saved: series.preset, onSaved: series.reload };
  return {
    target,
    preset: series.shown("cut_preset", ownPreset),
    setPreset,
    cut,
    settled: series.settled,
  };
}
