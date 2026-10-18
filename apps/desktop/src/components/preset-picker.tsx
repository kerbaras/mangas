import { useId } from "react";
import { CutStrip } from "@/components/cut-strip";
import { cn } from "@/lib/cn";
import type { Preset } from "@/lib/ipc";
import { PRESETS } from "@/lib/presets";

type Props = { value: Preset; onChange: (p: Preset) => void; compact?: boolean };

/** Radio group drawn as three strips: pick by how the sample strip gets cut, not by a label. */
export function PresetPicker({ value, onChange, compact }: Props) {
  const name = useId();
  return (
    <fieldset className="grid grid-cols-3 gap-2">
      <legend className="sr-only">Cut preset</legend>
      {PRESETS.map((p) => {
        const active = p.id === value;
        return (
          <label
            key={p.id}
            className={cn(
              "relative flex cursor-default gap-3 rounded-md border p-3 text-left transition-colors",
              "has-[:focus-visible]:ring-2 has-[:focus-visible]:ring-accent",
              active
                ? "border-accent bg-accent-soft/40"
                : "border-stroke bg-surface hover:bg-surface-2",
            )}
          >
            <input
              type="radio"
              name={name}
              value={p.id}
              checked={active}
              onChange={() => onChange(p.id)}
              className="sr-only"
            />
            <CutStrip preset={p} active={active} />
            <div className="min-w-0">
              <p className="text-body font-medium">{p.label}</p>
              <p className="mt-0.5 text-caption text-ink-3">
                {p.cuts.length + 1} pages
                {p.shrink ? `, shrink ≤ ${Math.round(p.shrink * 100)}%` : ""}
              </p>
              {!compact && <p className="mt-2 text-caption text-ink-2">{p.description}</p>}
            </div>
          </label>
        );
      })}
    </fieldset>
  );
}
