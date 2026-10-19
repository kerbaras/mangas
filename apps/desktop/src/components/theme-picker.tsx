import { Monitor, Moon, Sun } from "lucide-react";
import { type ReactNode, useId } from "react";
import { cn } from "@/lib/cn";
import type { Theme } from "@/lib/ipc";

const THEMES: { id: Theme; label: string; icon: ReactNode }[] = [
  { id: "light", label: "Light", icon: <Sun /> },
  { id: "dark", label: "Dark", icon: <Moon /> },
  { id: "system", label: "System", icon: <Monitor /> },
];

type Props = { value: Theme; onChange: (t: Theme) => void };

/** Three radio tiles; the pick applies at once. */
export function ThemePicker({ value, onChange }: Props) {
  const name = useId();
  return (
    <fieldset className="grid grid-cols-3 gap-2">
      <legend className="sr-only">Theme</legend>
      {THEMES.map((t) => (
        <label
          key={t.id}
          className={cn(
            "flex cursor-default items-center gap-2 rounded-md border px-3 py-2 transition-colors [&_svg]:size-4",
            "has-[:focus-visible]:ring-2 has-[:focus-visible]:ring-accent",
            t.id === value
              ? "border-accent bg-accent-soft/40 text-ink"
              : "border-stroke bg-surface text-ink-2 hover:bg-surface-2",
          )}
        >
          <input
            type="radio"
            name={name}
            value={t.id}
            checked={t.id === value}
            onChange={() => onChange(t.id)}
            className="sr-only"
          />
          {t.icon}
          {t.label}
        </label>
      ))}
    </fieldset>
  );
}
