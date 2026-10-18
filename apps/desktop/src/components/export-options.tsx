import { type Delivery, DeliveryPicker } from "@/components/delivery-picker";
import { DeviceSelect } from "@/components/device-select";
import { DevicesPending } from "@/components/devices-pending";
import { Field } from "@/components/field";
import { FormatPicker } from "@/components/format-picker";
import { OutputFolder } from "@/components/output-folder";
import { SmartCutSection } from "@/components/smart-cut-section";
import type { Async } from "@/hooks/use-async";
import type { ExportTarget } from "@/hooks/use-export-target";
import { formatLabel } from "@/lib/export-formats";
import type { DetectorsStatus, Preset } from "@/lib/ipc";

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
      />
      <OutputFolder {...p.output} />
      <DeliveryPicker value={p.delivery} onChange={p.onDelivery} format={target.format} />
    </div>
  );
}
