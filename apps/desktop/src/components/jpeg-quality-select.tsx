import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { JPEG_QUALITIES } from "@/lib/kindle-size";

type Props = {
  id: string;
  /** null: the device's own quality. */
  value: number | null;
  onChange: (quality: number | null) => void;
  /** Prefix each option with "JPEG quality:" where no field label names it. */
  named?: boolean;
};

/** The JPEG quality of a device profile's pages: its own, or a lower one for smaller books. */
export function JpegQualitySelect({ id, value, onChange, named = false }: Props) {
  const name = (v: string) => (named ? `JPEG quality: ${v}` : v);
  return (
    <Select
      value={value === null ? "profile" : String(value)}
      onValueChange={(v) => onChange(v === "profile" ? null : Number(v))}
    >
      <SelectTrigger id={id} aria-label="JPEG quality">
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        <SelectItem value="profile">{name(named ? "device default" : "Device default")}</SelectItem>
        {JPEG_QUALITIES.map((q) => (
          <SelectItem key={q} value={String(q)}>
            {name(String(q))}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}
