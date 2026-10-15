import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import type { Device } from "@/lib/ipc";

type Props = { devices: Device[]; value: string; onChange: (id: string) => void; id?: string };

export function DeviceSelect({ devices, value, onChange, id }: Props) {
  return (
    <Select value={value} onValueChange={onChange}>
      <SelectTrigger id={id} aria-label="Device">
        <SelectValue placeholder="Pick a device" />
      </SelectTrigger>
      <SelectContent>
        {devices.map((d) => (
          <SelectItem key={d.id} value={d.id}>
            {d.name}
            <span className="ml-2 text-caption text-ink-3 tabular-nums">
              {d.width}×{d.height}
            </span>
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}
