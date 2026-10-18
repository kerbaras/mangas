import { useCallback, useEffect, useRef, useState } from "react";
import { compact, type ExportPreset, getExportPreset, isEmptyPreset } from "@/lib/ipc.cut-preview";

export type Series = { source: string; id: string; title: string };

/** Export dialog fields a series preset can decide. */
export type PresetField = "profile" | "format" | "cut_preset";

/**
 * The saved export preset of `series`. A field the preset sets is shown from it until the user
 * picks another value in the dialog; core applies the rest (page height, trim, filters).
 */
export function useSeriesPreset(series: Series | null) {
  const key = series ? `${series.source}\u0000${series.id}` : null;
  const [preset, setPreset] = useState<ExportPreset | null>(null);
  const [touched, setTouched] = useState<ReadonlySet<PresetField>>(new Set());

  const latest = useRef(0);
  const reload = useCallback(() => {
    const request = ++latest.current;
    if (!key) return setPreset(null);
    const [source, id] = key.split("\u0000");
    // Only the newest lookup may set the preset (a series change or a reload after saving).
    const current = () => latest.current === request;
    getExportPreset(source, id).then(
      (p) => current() && setPreset(p && !isEmptyPreset(p) ? compact(p) : null),
      () => current() && setPreset(null),
    );
  }, [key]);

  // A newly picked series starts from the defaults until its own preset has loaded.
  useEffect(() => {
    setPreset(null);
    setTouched(new Set());
    reload();
  }, [reload]);

  const decides = (f: PresetField) => !touched.has(f) && preset?.[f] != null;
  return {
    preset,
    reload,
    shown: <T>(f: PresetField, own: T): T => (decides(f) ? (preset?.[f] as T) : own),
    touch: (f: PresetField) => setTouched((t) => new Set(t).add(f)),
  };
}
