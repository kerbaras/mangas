import { FolderInput } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";
import { ConfirmDialog } from "@/components/confirm-dialog";
import { Field } from "@/components/field";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Progress } from "@/components/ui/progress";
import { activeCount, useDownloads } from "@/hooks/use-downloads";
import { pickFolder } from "@/lib/dialogs";
import { type MoveProgress, moveDataDir, onMoveProgress } from "@/lib/ipc.storage";
import { notifyError } from "@/lib/notify";
import { loadSettings, useSettings } from "@/lib/settings";
import { formatBytes } from "@/lib/storage";

/** The data folder, moved (copy, verify, switch) rather than edited, so chapters follow it. */
export function DataFolderField({ onMoved }: { onMoved: () => void }) {
  const [settings] = useSettings();
  const downloading = activeCount(useDownloads()) > 0;
  const [target, setTarget] = useState<string | null>(null);
  const [progress, setProgress] = useState<MoveProgress | null>(null);

  const choose = async () => {
    const dir = await pickFolder("Move the data folder to", "~/Mangas");
    if (dir && dir !== settings.dataDir) setTarget(dir);
  };

  const move = async (to: string) => {
    setTarget(null);
    setProgress({ copied: 0, total: 0 });
    const unlisten = await onMoveProgress(setProgress);
    try {
      const moved = await moveDataDir(to);
      await loadSettings();
      const left = moved.leftovers.length
        ? ` Couldn't delete ${moved.leftovers.join(", ")}; delete it by hand.`
        : "";
      toast.success("Data folder moved", {
        description: `${moved.files} files (${formatBytes(moved.bytes)}) now live in ${moved.data_dir}.${left}`,
      });
    } catch (e: unknown) {
      notifyError("Couldn't move the data folder", e);
    } finally {
      unlisten();
      setProgress(null);
      onMoved();
    }
  };

  const hint = downloading
    ? "Wait for the downloads to finish before moving the data folder."
    : "Downloaded chapters and detector models live here; the library database, cover cache and logs stay in the app data folder. Moving copies everything, checks the copy, then deletes the old folder.";

  return (
    <Field label="Data folder" htmlFor="data-dir" hint={progress ? undefined : hint}>
      <div className="flex gap-2">
        <Input id="data-dir" readOnly value={settings.dataDir} className="font-mono text-caption" />
        <Button onClick={choose} disabled={downloading || progress !== null} className="shrink-0">
          <FolderInput /> Move…
        </Button>
      </div>
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
        title="Move the data folder?"
        description={`Chapters and detector models are copied to ${target ?? ""} and checked; only then the app switches to it and deletes the old copies.`}
        confirm="Move"
        onConfirm={() => target && void move(target)}
        onCancel={() => setTarget(null)}
      />
    </Field>
  );
}
