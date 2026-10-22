import { Field } from "@/components/field";
import { JpegQualitySelect } from "@/components/jpeg-quality-select";
import { SizeLevers } from "@/components/size-levers";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import type { BookBudget } from "@/lib/ipc.volumes";
import { budgetNote, PART_MB } from "@/lib/kindle-size";

type Props = {
  /** Each planned book against its budget, in plan order; null while estimating. */
  budgets: (BookBudget | null)[] | null;
  /** Emailed: the 50 MB budget, else 200 MB. */
  email: boolean;
  /** The device profile's stored JPEG quality; null: its own. */
  quality: number | null;
  onQuality: (q: number | null) => void;
  /** Part size in MB; null: one file. */
  splitMb: number | null;
  onSplit: (mb: number | null) => void;
  /** Write fewer chapters per book; absent when chapters of several series go in one. */
  onEvery?: (chapters: number) => void;
};

/** Export option: the estimated size against the delivery's budget, and the levers to fit it. */
export function KindleSizeField(p: Props) {
  const sized = (p.budgets ?? []).filter((b): b is BookBudget => b !== null);
  const note = sized.length > 0 ? budgetNote(sized) : null;
  const over = note?.over ?? 0;
  const whose =
    over === sized.length
      ? over > 1
        ? "Every book is"
        : "This book is"
      : `${over} of ${sized.length} books are`;
  return (
    <Field label="Size" htmlFor="export-quality">
      <p className="text-body text-ink-2" aria-live="polite">
        {note ? note.label : "Estimating…"}
      </p>
      {note && over > 0 && (
        <div role="alert" className="flex flex-col gap-2">
          <p className="text-caption text-accent-text">
            {whose} over the {note.budgetMb} MB Send to Kindle {p.email ? "email" : "web"} limit.
            Pick one to make it fit; nothing is re-encoded until you do.
          </p>
          {note.levers && (
            <SizeLevers
              levers={note.levers}
              splitMb={p.splitMb}
              onSplit={p.onSplit}
              onQuality={p.onQuality}
              onEvery={p.onEvery}
            />
          )}
        </div>
      )}
      <div className="flex flex-wrap items-center gap-3">
        <label htmlFor="export-split" className="flex items-center gap-2 text-body text-ink-2">
          <Switch
            id="export-split"
            aria-label="Split into parts"
            checked={p.splitMb !== null}
            onCheckedChange={(on) => p.onSplit(on ? PART_MB : null)}
          />
          Split into parts of
        </label>
        <Input
          type="number"
          min={1}
          value={p.splitMb ?? PART_MB}
          disabled={p.splitMb === null}
          onChange={(e) => p.onSplit(Math.max(1, Number(e.target.value) || 1))}
          aria-label="Part size in MB"
          className="w-20"
        />
        <span className="text-body text-ink-2">MB</span>
      </div>
      <JpegQualitySelect id="export-quality" value={p.quality} onChange={p.onQuality} named />
    </Field>
  );
}
