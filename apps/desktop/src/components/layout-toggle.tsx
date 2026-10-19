import { LayoutGrid, List } from "lucide-react";
import { Hint } from "@/components/hint";
import { cn } from "@/lib/cn";
import type { LibraryLayout } from "@/lib/library-view";

const options: { id: LibraryLayout; label: string; icon: typeof List }[] = [
  { id: "grid", label: "Grid", icon: LayoutGrid },
  { id: "list", label: "List", icon: List },
];

/** Segmented grid/list switch. */
export function LayoutToggle({
  value,
  onChange,
}: {
  value: LibraryLayout;
  onChange: (l: LibraryLayout) => void;
}) {
  return (
    <fieldset aria-label="Layout" className="flex h-8 items-center rounded-md bg-surface-2 p-0.5">
      {options.map(({ id, label, icon: Icon }) => (
        <Hint key={id} hint={label} focusable={false}>
          <button
            type="button"
            aria-label={label}
            aria-pressed={value === id}
            onClick={() => onChange(id)}
            className={cn(
              "flex size-7 items-center justify-center rounded-sm text-ink-2 transition-colors hover:text-ink [&_svg]:size-4",
              value === id && "bg-surface text-ink shadow-sm",
            )}
          >
            <Icon />
          </button>
        </Hint>
      ))}
    </fieldset>
  );
}
