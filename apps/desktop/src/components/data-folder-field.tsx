import { FolderInput } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";
import { ConfirmDialog } from "@/components/confirm-dialog";
import { Field } from "@/components/field";
import { PartialMoveAlert } from "@/components/partial-move-alert";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Progress } from "@/components/ui/progress";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { activeCount, useDownloads } from "@/hooks/use-downloads";
import { pickFolder } from "@/lib/dialogs";
import {
  type MoveProgress,
  moveDataDir,
  moveTargetPartial,
  onMoveProgress,
  type PartialCopy,
} from "@/lib/ipc.storage";
import { notifyError } from "@/lib/notify";
import { loadSettings, useSettings } from "@/lib/settings";
import { formatBytes, plural } from "@/lib/storage";

type Props = { onMoved: () => void; partial: PartialCopy | null };
/** A picked folder, with the partial copy a killed move left there (replaced by the move). */
type Target = { dir: string; partial: PartialCopy | null };

/** The data folder, moved (copy, verify, switch) rather than edited, so chapters follow it. */
export function DataFolderField({ onMoved, partial }: Props) {
  const [settings] = useSettings();
  const active = activeCount(useDownloads());
  const downloading = active > 0;
  const [target, setTarget] = useState<Target | null>(null);
  const [progress, setProgress] = useState<MoveProgress | null>(null);

  const choose = async () => {
    const dir = await pickFolder("Move the data folder to", "~/mangas");
    if (!dir || dir === settings.dataDir) return;
    try {
      setTarget({ dir, partial: await moveTargetPartial(dir) });
    } catch (e: unknown) {
      notifyError("Couldn't check that folder", e);
    }
  };

  const move = async ({ dir: to, partial: replace }: Target) => {
    setTarget(null);
    setProgress({ copied: 0, total: 0 });
    let unlisten: (() => void) | undefined;
    try {
      unlisten = await onMoveProgress(setProgress);
      const moved = await moveDataDir(to, replace !== null);
      await loadSettings();
      const left = moved.leftovers.length
        ? ` Couldn't delete ${moved.leftovers.join(", ")}; delete it by hand.`
        : "";
      toast.success("Data folder moved", {
        id: "data-folder-move",
        action: null,
        description: `${plural(moved.files, "file")} (${formatBytes(moved.bytes)}) now live in ${moved.data_dir}.${left}`,
      });
    } catch (e: unknown) {
      notifyError("Couldn't move the data folder", e, { id: "data-folder-move" });
    } finally {
      unlisten?.();
      setProgress(null);
      onMoved();
    }
  };

  const busy = `${plural(active, "download")} queued or running. Wait for ${active === 1 ? "it" : "them"} to finish before moving the data folder.`;
  const hint = downloading
    ? busy
    : "Downloaded chapters and detector models live here; the library database, cover cache and logs stay in the app data folder. Moving copies everything, checks the copy, then deletes the old folder.";

  const moveButton = (
    <Button onClick={choose} disabled={downloading || progress !== null} className="shrink-0">
      <FolderInput /> Move…
    </Button>
  );

  return (
    <Field label="Data folder" htmlFor="data-dir" hint={progress ? undefined : hint}>
      <div className="flex gap-2">
        <Input id="data-dir" readOnly value={settings.dataDir} className="font-mono text-caption" />
        {downloading ? (
          <Tooltip>
            <TooltipTrigger asChild>
              <span className="shrink-0">{moveButton}</span>
            </TooltipTrigger>
            <TooltipContent>{busy}</TooltipContent>
          </Tooltip>
        ) : (
          moveButton
        )}
      </div>
      {partial && !progress && <PartialMoveAlert copy={partial} onRemoved={onMoved} />}
      {progress && (
        <div className="flex flex-col gap-1">
          <Progress
            value={progress.copied}
            max={progress.total}
            aria-label="Moving the data folder"
          />
          <p className="text-caption text-ink-3">
            Copying {formatBytes(progress.copied)} of {formatBytes(progress.total)}… Keep the app
            open until it finishes.
          </p>
        </div>
      )}
      <ConfirmDialog
        open={target !== null}
        title={target?.partial ? "Replace the partial copy and move?" : "Move the data folder?"}
        description={
          target?.partial
            ? `${target.dir} holds a partial copy (${formatBytes(target.partial.bytes)}) left by an interrupted move. Everything in its chapters and models folders is deleted, including files added there by hand, then chapters and detector models are copied there again and checked before the app switches to it.`
            : `Chapters and detector models are copied to ${target?.dir ?? ""} and checked; only then the app switches to it and deletes the old copies.`
        }
        confirm={target?.partial ? "Replace and move" : "Move"}
        onConfirm={() => target && void move(target)}
        onCancel={() => setTarget(null)}
      />
    </Field>
  );
}
