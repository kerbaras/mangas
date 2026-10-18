import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import type { GroupCount } from "@/lib/chapter-groups";

const ALL = "\u0000all";

type Props = {
  groups: GroupCount[];
  /** The group shown, null for every group. */
  value: string | null;
  onChange: (group: string | null) => void;
};

/** Narrows the chapter list to one scanlation group; a saved group no longer listed stays pickable. */
export function GroupSelect({ groups, value, onChange }: Props) {
  const options =
    value && !groups.some((g) => g.name === value)
      ? [...groups, { name: value, chapters: 0 }]
      : groups;
  const total = groups.reduce((n, g) => n + g.chapters, 0);
  return (
    <Select value={value ?? ALL} onValueChange={(v) => onChange(v === ALL ? null : v)}>
      <SelectTrigger aria-label="Group" className="h-7 w-56">
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        <SelectItem value={ALL}>All groups ({total})</SelectItem>
        {options.map((g) => (
          <SelectItem key={g.name} value={g.name}>
            {g.name} ({g.chapters})
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}
