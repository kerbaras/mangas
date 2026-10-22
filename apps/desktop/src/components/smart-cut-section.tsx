import type { ReactNode } from "react";
import { DetectorSupportActions } from "@/components/detector-support";
import { Field } from "@/components/field";
import { PresetPicker } from "@/components/preset-picker";
import { Switch } from "@/components/ui/switch";
import type { DetectorSupport } from "@/hooks/use-detector-support";
import { detectorHint } from "@/lib/detector-hint";
import { detectorsUsable } from "@/lib/export-plan";
import type { Preset } from "@/lib/ipc";

type Props = {
  preset: Preset;
  onPreset: (p: Preset) => void;
  support: DetectorSupport;
  detectors: boolean;
  onDetectors: (on: boolean) => void;
  /** The series' smart-cut options entry. */
  children?: ReactNode;
};

/**
 * Smart cut in the export dialog: the cut preset, the series' smart-cut options and the detectors
 * switch with its download block.
 */
export function SmartCutSection({
  preset,
  onPreset,
  support,
  detectors,
  onDetectors,
  children,
}: Props) {
  const usable = detectorsUsable(support.status);
  return (
    <section aria-label="Smart cut" className="flex flex-col gap-3">
      <Field
        label="Cut preset"
        hint="Long strips are cut into pages at gutters; a preset decides how hard to avoid cutting through art."
      >
        <PresetPicker value={preset} onChange={onPreset} compact />
      </Field>
      {children}
      <div className="flex items-start justify-between gap-4">
        <div>
          <label htmlFor="detectors" className="text-body font-medium">
            Use detectors
          </label>
          <p className="mt-0.5 text-caption text-ink-3">{detectorHint(support.status)}</p>
        </div>
        <Switch
          id="detectors"
          aria-label="Use detectors"
          checked={usable && detectors}
          onCheckedChange={onDetectors}
          disabled={!usable}
        />
      </div>
      <DetectorSupportActions support={support} />
    </section>
  );
}
