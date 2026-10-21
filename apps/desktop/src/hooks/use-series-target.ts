import { useState } from "react";
import { toast } from "sonner";
import { useExportTarget } from "@/hooks/use-export-target";
import { type Series, useSeriesPreset } from "@/hooks/use-series-preset";
import { effectiveFormat } from "@/lib/export-formats";
import type { ExportPick } from "@/lib/export-name";
import { errorMessage } from "@/lib/format";
import type { Preset } from "@/lib/ipc";
import { setExportPreset } from "@/lib/ipc.cut-preview";
import type { ReadingDirection } from "@/lib/ipc.domain";
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
  const key = picks.map((p) => `${p.manga.source}/${p.manga.id}`).join("|");
  const [own, setOwn] = useState<{ key: string; direction: ReadingDirection } | null>(null);
  // A pick holds for the series it was made for; other picks start from their own way.
  const ownDirection = own?.key === key ? own.direction : null;
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
    /** Picked here; null lets the series preset, else the source, decide for each series. */
    direction: ownDirection,
    /** What one series' book turns by; null when several series turn the way they all do. */
    shownDirection: info
      ? (ownDirection ?? series.preset?.direction ?? pick?.manga.direction ?? "ltr")
      : ownDirection,
    directionSaved:
      series.preset?.direction != null &&
      (ownDirection ?? series.preset.direction) === series.preset.direction,
    /** Picking a direction for one series saves it as that series' setting. */
    setDirection: (d: ReadingDirection) => {
      const before = own;
      const picked = { key, direction: d };
      setOwn(picked);
      if (!info) return;
      const saved = { ...series.preset, direction: d };
      setExportPreset(info.source, info.id, saved).then(series.reload, (e: unknown) => {
        // A later pick wins over this failed save.
        setOwn((now) => (now === picked ? before : now));
        toast.error("Couldn't save the page turn for this series", {
          description: errorMessage(e),
        });
      });
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

export type SeriesTarget = ReturnType<typeof useSeriesTarget>["target"];
