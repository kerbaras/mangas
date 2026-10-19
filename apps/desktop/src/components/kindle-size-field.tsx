import { Field } from "@/components/field";
import { JpegQualitySelect } from "@/components/jpeg-quality-select";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import type { Async } from "@/hooks/use-async";
import { useSizeEstimate } from "@/hooks/use-size-estimate";
import type { Preset } from "@/lib/ipc";
import type { VolumePlan } from "@/lib/ipc.volumes";
import { PART_MB, SEND_TO_KINDLE_MB, sizeNote } from "@/lib/kindle-size";

type Props = {
  plans: Async<VolumePlan[]>;
  device: string;
  preset: Preset;
  /** The device profile's stored JPEG quality; null: its own. */
  quality: number | null;
  onQuality: (q: number | null) => void;
  /** Part size in MB; null: one file. */
  splitMb: number | null;
  onSplit: (mb: number | null) => void;
};

/** Export option: the estimated size, a warning over the Send-to-Kindle limit, and the fixes. */
export function KindleSizeField(p: Props) {
  const { plans, splitMb } = p;
  const estimate = useSizeEstimate({
    plans,
    device: p.device,
    preset: p.preset,
    quality: p.quality,
  });
  const books = plans.kind === "ready" ? plans.data.length : 0;
  const note = estimate && sizeNote(estimate, books, splitMb);
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
            checked={splitMb !== null}
            onCheckedChange={(on) => p.onSplit(on ? PART_MB : null)}
          />
          Split into parts of
        </label>
        <Input
          type="number"
          min={1}
          value={splitMb ?? PART_MB}
          disabled={splitMb === null}
          onChange={(e) => p.onSplit(Math.max(1, Number(e.target.value) || 1))}
          aria-label="Part size in MB"
          className="w-20"
        />
        <span className="text-body text-ink-2">MB</span>
      </div>
      <JpegQualitySelect id="export-quality" value={p.quality} onChange={p.onQuality} named />
    </Field>
  );
}
