import { Field } from "@/components/field";
import { Input } from "@/components/ui/input";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import type { BundleMode } from "@/lib/export-plan";

type Props = {
  mode: BundleMode;
  onMode: (m: BundleMode) => void;
  size: number;
  onSize: (n: number) => void;
};

/** How the chapters become files: one each, by provider volume, every N chapters, one book. */
export function BundleField({ mode, onMode, size, onSize }: Props) {
  return (
    <Field label="Bundle">
      <div className="flex flex-wrap items-center gap-3">
        <Tabs value={mode} onValueChange={(v) => onMode(v as BundleMode)}>
          <TabsList aria-label="Bundle">
            <TabsTrigger value="per_chapter">Per chapter</TabsTrigger>
            <TabsTrigger value="by_volume">By volume</TabsTrigger>
            <TabsTrigger value="every">Every N</TabsTrigger>
            <TabsTrigger value="single">One book</TabsTrigger>
          </TabsList>
        </Tabs>
        {mode === "every" && (
          <Input
            type="number"
            min={1}
            value={size}
            onChange={(e) => onSize(Math.max(1, Number(e.target.value) || 1))}
            aria-label="Chapters per book"
            className="w-20"
          />
        )}
      </div>
    </Field>
  );
}
