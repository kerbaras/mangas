import { Tags } from "lucide-react";
import { AutoDownloadSwitch } from "@/components/auto-download-switch";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuCheckboxItem,
  DropdownMenuContent,
  DropdownMenuLabel,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  type Category,
  type LibraryEntry,
  READING_STATUSES,
  type ReadingStatus,
} from "@/lib/ipc.library";
import { notifyError } from "@/lib/notify";

type Props = {
  entry: LibraryEntry;
  categories: Category[];
  onStatus: (e: LibraryEntry, s: ReadingStatus) => Promise<void>;
  onToggle: (id: number, on: boolean) => Promise<void>;
};

/** Reading status and categories of a followed title, on its details screen. */
export function LibraryControls({ entry, categories, onStatus, onToggle }: Props) {
  const names = categories.filter((c) => entry.categories.includes(c.id)).map((c) => c.name);
  return (
    <div className="mt-3 flex flex-wrap items-center gap-x-4 gap-y-2">
      <Select
        value={entry.reading_status}
        onValueChange={(v) =>
          onStatus(entry, v as ReadingStatus).catch((e: unknown) =>
            notifyError("Couldn't set status", e),
          )
        }
      >
        <SelectTrigger aria-label="Reading status" className="h-7 w-36">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {READING_STATUSES.map((s) => (
            <SelectItem key={s.id} value={s.id}>
              {s.label}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button size="sm" className="max-w-72">
            <Tags />
            <span className="truncate">{names.length ? names.join(", ") : "Add to category"}</span>
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="start">
          {categories.length === 0 && (
            <DropdownMenuLabel>No categories yet. Add them from the library.</DropdownMenuLabel>
          )}
          {categories.map((c) => (
            <DropdownMenuCheckboxItem
              key={c.id}
              checked={entry.categories.includes(c.id)}
              onSelect={(e) => e.preventDefault()}
              onCheckedChange={(on) =>
                onToggle(c.id, on === true).catch((e: unknown) =>
                  notifyError("Couldn't change categories", e),
                )
              }
            >
              {c.name}
            </DropdownMenuCheckboxItem>
          ))}
        </DropdownMenuContent>
      </DropdownMenu>
      <AutoDownloadSwitch key={`${entry.source}:${entry.id}`} entry={entry} />
    </div>
  );
}
