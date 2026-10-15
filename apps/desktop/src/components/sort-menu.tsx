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
import type { Manga } from "@/lib/ipc";

export type LibrarySort = "title" | "status" | "source";

const options: { id: LibrarySort; label: string }[] = [
  { id: "title", label: "Title" },
  { id: "status", label: "Status" },
  { id: "source", label: "Source" },
];

export function sortLibrary(items: Manga[], sort: LibrarySort): Manga[] {
  return [...items].sort(
    (a, b) =>
      (sort === "title" ? 0 : a[sort].localeCompare(b[sort])) || a.title.localeCompare(b.title),
  );
}

export function SortMenu({
  value,
  onChange,
}: {
  value: LibrarySort;
  onChange: (s: LibrarySort) => void;
}) {
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="ghost" size="icon" aria-label="Sort">
          <ArrowUpDown />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        <DropdownMenuLabel>Sort by</DropdownMenuLabel>
        <DropdownMenuRadioGroup value={value} onValueChange={(v) => onChange(v as LibrarySort)}>
          {options.map((o) => (
            <DropdownMenuRadioItem key={o.id} value={o.id}>
              {o.label}
            </DropdownMenuRadioItem>
          ))}
        </DropdownMenuRadioGroup>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
