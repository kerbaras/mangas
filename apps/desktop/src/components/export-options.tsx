import { type Delivery, DeliveryPicker } from "@/components/delivery-picker";
import { DeviceSelect } from "@/components/device-select";
import { DevicesPending } from "@/components/devices-pending";
import { Field } from "@/components/field";
import { FormatPicker } from "@/components/format-picker";
import { OutputFolder } from "@/components/output-folder";
import { SmartCutButton } from "@/components/smart-cut-preview";
import { SmartCutSection } from "@/components/smart-cut-section";
import type { Async } from "@/hooks/use-async";
import type { ExportTarget } from "@/hooks/use-export-target";
import type { Series } from "@/hooks/use-series-preset";
import { chapterLabel } from "@/lib/chapters";
import { formatLabel } from "@/lib/export-formats";
import type { Chapter, DetectorsStatus, Preset } from "@/lib/ipc";
import type { ExportPreset } from "@/lib/ipc.cut-preview";

type Props = {
  target: ExportTarget;
  preset: Preset;
  onPreset: (p: Preset) => void;
  detectorsStatus: Async<DetectorsStatus>;
  detectors: boolean;
  onDetectors: (on: boolean) => void;
  output: { value: string; fallback: string; onChange: (dir: string) => void };
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
          target.recommended &&
          `${target.recommended.name} reads ${formatLabel(target.recommended.format)} best.`
        }
      >
        <FormatPicker value={target.format} onChange={target.setFormat} compact />
      </Field>
      <SmartCutSection
        preset={p.preset}
        onPreset={p.onPreset}
        status={p.detectorsStatus}
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
      <OutputFolder {...p.output} />
      <DeliveryPicker value={p.delivery} onChange={p.onDelivery} format={target.format} />
    </div>
  );
}
