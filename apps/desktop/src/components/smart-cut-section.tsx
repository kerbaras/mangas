import { Field } from "@/components/field";
import { PresetPicker } from "@/components/preset-picker";
import { Switch } from "@/components/ui/switch";
import type { Async } from "@/hooks/use-async";
import { detectorHint } from "@/lib/detector-hint";
import { detectorsUsable } from "@/lib/export-plan";
import type { DetectorsStatus, Preset } from "@/lib/ipc";

type Props = {
  preset: Preset;
  onPreset: (p: Preset) => void;
  status: Async<DetectorsStatus>;
  detectors: boolean;
  onDetectors: (on: boolean) => void;
};

/**
 * Smart cut in the export dialog: the cut preset and the detectors switch. Per-series smart-cut
 * options (preview, presets) belong in this section too, next to the preset picker.
 */
export function SmartCutSection({ preset, onPreset, status, detectors, onDetectors }: Props) {
  const usable = detectorsUsable(status);
  return (
    <section aria-label="Smart cut" className="flex flex-col gap-3">
      <Field
        label="Cut preset"
        hint="Long strips are cut into pages at gutters; a preset decides how hard to avoid cutting through art."
      >
        <PresetPicker value={preset} onChange={onPreset} compact />
      </Field>
      <div className="flex items-start justify-between gap-4">
        <div>
          <label htmlFor="detectors" className="text-body font-medium">
            Use detectors
          </label>
          <p className="mt-0.5 text-caption text-ink-3">{detectorHint(status)}</p>
        </div>
        <Switch
          id="detectors"
          checked={usable && detectors}
          onCheckedChange={onDetectors}
          disabled={!usable}
        />
      </div>
    </section>
  );
}
