import { type Delivery, DeliveryPicker } from "@/components/delivery-picker";
import { DeviceSelect } from "@/components/device-select";
import { DevicesPending } from "@/components/devices-pending";
import { DirectionField } from "@/components/direction-field";
import { Field } from "@/components/field";
import { FormatPicker } from "@/components/format-picker";
import { KindleSizeField } from "@/components/kindle-size-field";
import { OutputFolder } from "@/components/output-folder";
import { ReplaceSwitch } from "@/components/replace-switch";
import { SmartCutButton } from "@/components/smart-cut-preview";
import { SmartCutSection } from "@/components/smart-cut-section";
import type { DetectorSupport } from "@/hooks/use-detector-support";
import type { Series } from "@/hooks/use-series-preset";
import type { SeriesTarget } from "@/hooks/use-series-target";
import { chapterLabel } from "@/lib/chapters";
import { formatLabel } from "@/lib/export-formats";
import type { Chapter, Preset } from "@/lib/ipc";
import type { ExportPreset } from "@/lib/ipc.cut-preview";
import type { BookBudget } from "@/lib/ipc.volumes";

type Props = {
  /** `format` is the one this delivery writes; `reason` is the Kindle route rule. */
  target: SeriesTarget & { reason: string | null };
  preset: Preset;
  onPreset: (p: Preset) => void;
  detectorSupport: DetectorSupport;
  detectors: boolean;
  onDetectors: (on: boolean) => void;
  output: { value: string; fallback: string; onChange: (dir: string) => void };
  /** The planned books, sized against the Send-to-Kindle limit, and how to shrink them. */
  size: {
    budgets: (BookBudget | null)[] | null;
    splitMb: number | null;
    onSplit: (mb: number | null) => void;
    onEvery?: (chapters: number) => void;
  };
  delivery: Delivery;
  onDelivery: (d: Delivery) => void;
  /** The series whose smart-cut options can be previewed and saved (none: several series). */
  cut: {
    series: Series | null;
    chapters: Chapter[];
    saved: ExportPreset | null;
    onSaved: () => void;
  };
};

/** The export dialog's "how" column: profile, format, smart cut, folder and delivery. */
export function ExportOptions(p: Props) {
  const { target } = p;
  return (
    <div className="flex flex-col gap-4">
      <Field label="Device" htmlFor="export-device">
        {target.devices.kind === "ready" ? (
          <DeviceSelect
            id="export-device"
            devices={target.devices.data}
            value={target.device}
            onChange={target.setDevice}
          />
        ) : (
          <DevicesPending state={target.devices} />
        )}
      </Field>
      <Field
        label="Format"
        hint={
          target.reason
            ? `${target.reason}.`
            : target.recommended &&
              `${target.recommended.name} reads ${formatLabel(target.recommended.format)} best.`
        }
      >
        <FormatPicker value={target.format} onChange={target.setFormat} compact />
      </Field>
      <DirectionField
        value={target.shownDirection}
        saved={target.directionSaved}
        series={p.cut.series !== null}
        onChange={target.setDirection}
      />
      <SmartCutSection
        preset={p.preset}
        onPreset={p.onPreset}
        support={p.detectorSupport}
        detectors={p.detectors}
        onDetectors={p.onDetectors}
      >
        <SmartCutButton
          series={p.cut.series}
          chapters={p.cut.chapters.flatMap((c) =>
            c.local_path ? [{ id: c.id, label: chapterLabel(c), dir: c.local_path }] : [],
          )}
          device={target.recommended}
          format={target.format}
          cutPreset={p.preset}
          detectors={p.detectors}
          saved={p.cut.saved}
          onSaved={p.cut.onSaved}
        />
      </SmartCutSection>
      <KindleSizeField
        {...p.size}
        email={p.delivery === "email"}
        quality={target.quality}
        onQuality={target.setQuality}
      />
      <OutputFolder {...p.output} />
      <ReplaceSwitch on={target.replace} onChange={target.setReplace} />
      <DeliveryPicker
        value={p.delivery}
        onChange={p.onDelivery}
        format={target.formatFor("email")}
      />
    </div>
  );
}
