import { Field } from "@/components/field";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import type { Preset } from "@/lib/ipc";
import type { ExportPreset } from "@/lib/ipc.cut-preview";
import { isPreset, PRESETS, presetInfo } from "@/lib/presets";

type Props = {
  value: ExportPreset;
  onChange: (v: ExportPreset) => void;
  /** The cut preset the export screen uses when the form leaves it unset. */
  basePreset: Preset;
  /** Grey e-ink profile: its filter defaults differ (see `ImageSettings::for_device`). */
  grayscale: boolean;
};

type RangeProps = {
  id: string;
  label: string;
  value: number | undefined;
  fallback: number;
  min: number;
  max: number;
  step: number;
  show: (v: number) => string;
  onChange: (v: number) => void;
};

const pct = (v: number) => `${Math.round(v * 100)}%`;

function Range({ id, label, value, fallback, min, max, step, show, onChange }: RangeProps) {
  const current = value ?? fallback;
  return (
    <div className="flex flex-col gap-1">
      <div className="flex items-baseline justify-between">
        <label htmlFor={id} className="text-body font-medium">
          {label}
        </label>
        <span className="text-caption tabular-nums text-ink-3">
          {show(current)}
          {value === undefined ? " (default)" : ""}
        </span>
      </div>
      <input
        id={id}
        type="range"
        min={min}
        max={max}
        step={step}
        value={current}
        onChange={(e) => onChange(Number(e.target.value))}
        className="w-full accent-[var(--color-accent)]"
      />
    </div>
  );
}

function Toggle(props: {
  id: string;
  label: string;
  hint: string;
  checked: boolean;
  onChange: (v: boolean) => void;
}) {
  return (
    <div className="flex items-start justify-between gap-4">
      <div>
        <label htmlFor={props.id} className="text-body font-medium">
          {props.label}
        </label>
        <p className="mt-0.5 text-caption text-ink-3">{props.hint}</p>
      </div>
      <Switch id={props.id} checked={props.checked} onCheckedChange={props.onChange} />
    </div>
  );
}

/** Smart-cut and page options of an export preset; unset fields show their default. */
export function SmartCutForm({ value, onChange, basePreset, grayscale }: Props) {
  const set = (patch: ExportPreset) => onChange({ ...value, ...patch });
  const cut = value.cut_preset ?? basePreset;
  return (
    <div className="flex flex-col gap-4">
      <Field label="Cut preset" htmlFor="cut-preset">
        <Select value={cut} onValueChange={(p) => isPreset(p) && set({ cut_preset: p })}>
          <SelectTrigger id="cut-preset" aria-label="Cut preset">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {PRESETS.map((p) => (
              <SelectItem key={p.id} value={p.id}>
                {p.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </Field>
      <Range
        id="page-height"
        label="Target page height"
        value={value.page_height}
        fallback={1}
        min={0.5}
        max={1}
        step={0.05}
        show={(v) => `${pct(v)} of the screen`}
        onChange={(v) => set({ page_height: v })}
      />
      <Range
        id="overlap"
        label="Overlap after a forced cut"
        value={value.overlap}
        fallback={0.075}
        min={0}
        max={0.25}
        step={0.005}
        show={(v) => `${(v * 100).toFixed(1)}%`}
        onChange={(v) => set({ overlap: v })}
      />
      <Range
        id="max-shrink"
        label="Shrink to avoid a cut"
        value={value.max_shrink}
        fallback={1 + presetInfo(cut).shrink}
        min={1}
        max={1.5}
        step={0.05}
        show={(v) => `up to ${pct(v - 1)}`}
        onChange={(v) => set({ max_shrink: v })}
      />
      <Toggle
        id="trim-margins"
        label="Trim margins"
        hint="Crops blank borders so the art fills more of the screen."
        checked={value.trim_margins ?? false}
        onChange={(v) => set({ trim_margins: v })}
      />
      <Range
        id="contrast"
        label="Contrast"
        value={value.contrast}
        fallback={1.1}
        min={0.5}
        max={2}
        step={0.05}
        show={(v) => v.toFixed(2)}
        onChange={(v) => set({ contrast: v })}
      />
      <Range
        id="gamma"
        label="Gamma"
        value={value.gamma}
        fallback={grayscale ? 0.9 : 1}
        min={0.5}
        max={2}
        step={0.05}
        show={(v) => v.toFixed(2)}
        onChange={(v) => set({ gamma: v })}
      />
      {grayscale && (
        <Toggle
          id="sharpen"
          label="Sharpen"
          hint="Crisper line art on grey e-ink screens."
          checked={value.sharpen ?? true}
          onChange={(v) => set({ sharpen: v })}
        />
      )}
    </div>
  );
}
