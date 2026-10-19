import { Field } from "@/components/field";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import type { Async } from "@/hooks/use-async";
import { useSizeEstimate } from "@/hooks/use-size-estimate";
import type { Preset } from "@/lib/ipc";
import type { VolumePlan } from "@/lib/ipc.volumes";
import {
  JPEG_QUALITIES,
  PART_MB,
  SEND_TO_KINDLE_MB,
  type Shrink,
  sizeNote,
} from "@/lib/kindle-size";

type Props = {
  plans: Async<VolumePlan[]>;
  device: string;
  preset: Preset;
  value: Shrink;
  onChange: (s: Shrink) => void;
};

/** Export option: the estimated size, a warning over the Send-to-Kindle limit, and the fixes. */
export function KindleSizeField({ plans, device, preset, value, onChange }: Props) {
  const estimate = useSizeEstimate({ plans, device, preset, quality: value.quality });
  const books = plans.kind === "ready" ? plans.data.length : 0;
  const note = estimate && sizeNote(estimate, books, value.splitMb);
  return (
    <Field label="Size" htmlFor="export-quality">
      <p className="text-body text-ink-2" aria-live="polite">
        {note ? note.label : "Estimating…"}
      </p>
      {note?.over && (
        <p role="alert" className="text-caption text-accent-text">
          Send to Kindle email refuses files over {SEND_TO_KINDLE_MB} MB. Split it into parts or
          lower the JPEG quality.
        </p>
      )}
      <div className="flex flex-wrap items-center gap-3">
        <label htmlFor="export-split" className="flex items-center gap-2 text-body text-ink-2">
          <Switch
            id="export-split"
            checked={value.splitMb !== null}
            onCheckedChange={(on) => onChange({ ...value, splitMb: on ? PART_MB : null })}
          />
          Split into parts of
        </label>
        <Input
          type="number"
          min={1}
          value={value.splitMb ?? PART_MB}
          disabled={value.splitMb === null}
          onChange={(e) =>
            onChange({ ...value, splitMb: Math.max(1, Number(e.target.value) || 1) })
          }
          aria-label="Part size in MB"
          className="w-20"
        />
        <span className="text-body text-ink-2">MB</span>
      </div>
      <Select
        value={value.quality === null ? "profile" : String(value.quality)}
        onValueChange={(v) => onChange({ ...value, quality: v === "profile" ? null : Number(v) })}
      >
        <SelectTrigger id="export-quality" aria-label="JPEG quality">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="profile">JPEG quality: device default</SelectItem>
          {JPEG_QUALITIES.map((q) => (
            <SelectItem key={q} value={String(q)}>
              JPEG quality: {q}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </Field>
  );
}
