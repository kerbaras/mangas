import { Hint } from "@/components/hint";
import { Button } from "@/components/ui/button";
import type { ExportRecord } from "@/lib/ipc.send";
import { deliveryLabel } from "@/lib/send-labels";

type Props = { records: ExportRecord[]; current: string | null; onPick: (file: string) => void };

/** Books exported before (newest first), each with where it was sent; "Send" picks one. */
export function RecentExports({ records, current, onPick }: Props) {
  if (records.length === 0) return null;
  return (
    <div className="flex flex-col gap-1.5">
      <h3 className="text-body font-medium">Recent exports</h3>
      <ul className="divide-y divide-stroke rounded-md border border-stroke">
        {records.map((r) => (
          <li key={r.id} className="flex items-center justify-between gap-3 px-3 py-2">
            <div className="min-w-0">
              <Hint hint={r.file}>
                <p className="truncate text-body">{r.title || r.file}</p>
              </Hint>
              <p className="truncate text-caption text-ink-3">
                {r.exists ? deliveryLabel(r) : "File no longer on disk"}
              </p>
            </div>
            <Button
              size="sm"
              variant="ghost"
              disabled={!r.exists || r.file === current}
              onClick={() => onPick(r.file)}
              aria-label={`Send ${r.title || r.file}`}
            >
              Send…
            </Button>
          </li>
        ))}
      </ul>
    </div>
  );
}
