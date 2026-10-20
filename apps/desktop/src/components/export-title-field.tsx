import { Field } from "@/components/field";
import { Input } from "@/components/ui/input";
import type { Async } from "@/hooks/use-async";
import type { VolumePlan } from "@/lib/ipc.volumes";

type Props = { value: string; onChange: (title: string) => void; plans: Async<VolumePlan[]> };

/** The book's title, which is also its file name; the planned title shows until one is typed. */
export function ExportTitleField({ value, onChange, plans }: Props) {
  return (
    <Field label="Title" htmlFor="export-title" hint="Also the file name.">
      <Input
        id="export-title"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={plans.kind === "ready" ? (plans.data[0]?.title ?? "") : ""}
      />
    </Field>
  );
}
