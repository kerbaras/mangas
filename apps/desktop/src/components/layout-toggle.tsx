import { LayoutGrid, List } from "lucide-react";
import { Hint } from "@/components/hint";
import type { MessageKey } from "@/i18n/catalog";
import { useT } from "@/i18n/use-t";
import { cn } from "@/lib/cn";
import type { LibraryLayout } from "@/lib/library-view";

const options: { id: LibraryLayout; label: MessageKey; icon: typeof List }[] = [
  { id: "grid", label: "library.grid", icon: LayoutGrid },
  { id: "list", label: "library.list", icon: List },
];

/** Segmented grid/list switch. */
export function LayoutToggle({
  value,
  onChange,
}: {
  value: LibraryLayout;
  onChange: (l: LibraryLayout) => void;
}) {
  const t = useT();
  return (
    <fieldset
      aria-label={t("library.layout")}
      className="flex h-8 items-center rounded-md bg-surface-2 p-0.5"
    >
      {options.map(({ id, label, icon: Icon }) => (
        <Hint key={id} hint={t(label)} focusable={false}>
          <button
            type="button"
            aria-label={t(label)}
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
