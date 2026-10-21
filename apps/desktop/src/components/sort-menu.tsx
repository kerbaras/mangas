import { ArrowUpDown } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuLabel,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { useT } from "@/i18n/use-t";
import { type LibrarySort, SORTS } from "@/lib/library-view";

export function SortMenu({
  value,
  onChange,
}: {
  value: LibrarySort;
  onChange: (s: LibrarySort) => void;
}) {
  const t = useT();
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="ghost" size="icon" aria-label={t("library.sort")}>
          <ArrowUpDown />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        <DropdownMenuLabel>{t("library.sortBy")}</DropdownMenuLabel>
        <DropdownMenuRadioGroup value={value} onValueChange={(v) => onChange(v as LibrarySort)}>
          {SORTS.map((o) => (
            <DropdownMenuRadioItem key={o.id} value={o.id}>
              {o.label}
            </DropdownMenuRadioItem>
          ))}
        </DropdownMenuRadioGroup>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
