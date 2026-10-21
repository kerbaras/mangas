import { Field } from "@/components/field";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import type { ReadingDirection } from "@/lib/ipc.domain";

const EACH = "each";

type Props = {
  /** Null when several series turn the way they all do (else left to right). */
  value: ReadingDirection | null;
  /** The one series picked has this direction saved as its setting. */
  saved: boolean;
  /** One series is picked, so a pick is saved for it. */
  series: boolean;
  onChange: (d: ReadingDirection) => void;
};

/** Export option: which way the book's pages turn (right to left for manga). */
export function DirectionField({ value, saved, series, onChange }: Props) {
  const hint = !series
    ? "The way all these series turn, else left to right, unless you pick one."
    : saved
      ? "Saved for this series."
      : "From the source; picking one saves it for this series.";
  return (
    <Field label="Page turn" htmlFor="export-direction" hint={hint}>
      <Select
        value={value ?? EACH}
        onValueChange={(v) => v !== EACH && onChange(v as ReadingDirection)}
      >
        <SelectTrigger id="export-direction" aria-label="Page turn">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {value === null && <SelectItem value={EACH}>As the series read</SelectItem>}
          <SelectItem value="ltr">Left to right</SelectItem>
          <SelectItem value="rtl">Right to left (manga)</SelectItem>
        </SelectContent>
      </Select>
    </Field>
  );
}
