import { useId } from "react";
import { cn } from "@/lib/cn";
import { FORMATS } from "@/lib/export-formats";
import type { ExportFormat } from "@/lib/ipc";

type Props = { value: ExportFormat; onChange: (f: ExportFormat) => void; compact?: boolean };

/** Radio group of the export file formats, drawn as cards like the preset picker. */
export function FormatPicker({ value, onChange, compact }: Props) {
  const name = useId();
  return (
    <fieldset className="grid grid-cols-2 gap-2">
      <legend className="sr-only">File format</legend>
      {FORMATS.map((f) => {
        const active = f.id === value;
        return (
          <label
            key={f.id}
            className={cn(
              "relative flex cursor-default flex-col rounded-md border p-3 text-left transition-colors",
              "has-[:focus-visible]:ring-2 has-[:focus-visible]:ring-accent",
              active
                ? "border-accent bg-accent-soft/40"
                : "border-stroke bg-surface hover:bg-surface-2",
            )}
          >
            <input
              type="radio"
              name={name}
              value={f.id}
              checked={active}
              onChange={() => onChange(f.id)}
              className="sr-only"
            />
            <span className="text-body font-medium">{f.label}</span>
            {!compact && <span className="mt-0.5 text-caption text-ink-3">{f.description}</span>}
          </label>
        );
      })}
    </fieldset>
  );
}
