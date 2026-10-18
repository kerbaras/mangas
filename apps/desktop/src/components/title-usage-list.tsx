import { Trash2 } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";
import { ConfirmDialog } from "@/components/confirm-dialog";
import { Button } from "@/components/ui/button";
import { Progress } from "@/components/ui/progress";
import { deleteDownloads, type TitleUsage } from "@/lib/ipc.storage";
import { notifyError } from "@/lib/notify";
import { chaptersLabel, formatBytes } from "@/lib/storage";

type Props = { titles: TitleUsage[]; onChange: () => void };

/** Each title's downloads, largest first, with a bar relative to the largest. */
export function TitleUsageList({ titles, onChange }: Props) {
  const [pending, setPending] = useState<TitleUsage | null>(null);
  if (titles.length === 0) {
    return <p className="text-caption text-ink-3">No downloaded chapters yet.</p>;
  }
  const largest = titles[0]?.bytes ?? 0;

  const remove = async (t: TitleUsage) => {
    setPending(null);
    try {
      const deleted = await deleteDownloads(t.source, t.manga_id);
      toast.success(`Deleted the downloads of ${t.title}`, {
        description: `${chaptersLabel(deleted.chapters)}, ${formatBytes(deleted.bytes)} freed.${
          deleted.kept.length
            ? ` ${deleted.kept.length} folders outside the data folder were left alone.`
            : ""
        }`,
      });
    } catch (e: unknown) {
      notifyError(`Couldn't delete the downloads of ${t.title}`, e);
    }
    onChange();
  };

  return (
    <>
      <ul className="flex flex-col divide-y divide-stroke rounded-md border border-stroke bg-surface">
        {titles.map((t) => (
          <li key={`${t.source}:${t.manga_id}`} className="flex items-center gap-3 px-3 py-2">
            <div className="flex min-w-0 flex-1 flex-col gap-1">
              <div className="flex items-baseline justify-between gap-3">
                <span className="truncate text-body">{t.title}</span>
                <span className="shrink-0 text-caption tabular-nums text-ink-3">
                  {chaptersLabel(t.chapters)} · {formatBytes(t.bytes)}
                </span>
              </div>
              <Progress value={t.bytes} max={largest} aria-label={`${t.title} disk usage`} />
            </div>
            {t.source !== "local" && (
              <Button
                variant="ghost"
                size="icon"
                aria-label={`Delete the downloads of ${t.title}`}
                title="Delete downloads"
                onClick={() => setPending(t)}
              >
                <Trash2 />
              </Button>
            )}
          </li>
        ))}
      </ul>
      <ConfirmDialog
        open={pending !== null}
        title={`Delete the downloads of ${pending?.title ?? ""}?`}
        description={`${chaptersLabel(pending?.chapters ?? 0)} (${formatBytes(pending?.bytes ?? 0)}) are deleted from disk. The title and its chapter list stay in your library, so you can download them again.`}
        confirm="Delete downloads"
        onConfirm={() => pending && void remove(pending)}
        onCancel={() => setPending(null)}
      />
    </>
  );
}
