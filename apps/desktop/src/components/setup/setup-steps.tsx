import { FolderInput } from "lucide-react";
import { useState } from "react";
import { DeviceSelect } from "@/components/device-select";
import { DevicesPending } from "@/components/devices-pending";
import { ProvidersList } from "@/components/providers-list";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Progress } from "@/components/ui/progress";
import { useAsync } from "@/hooks/use-async";
import { parentFolder } from "@/lib/chapter-folder";
import { pickFolder } from "@/lib/dialogs";
import { listDevices } from "@/lib/ipc";
import { type MoveProgress, moveDataDir, onMoveProgress } from "@/lib/ipc.storage";
import { notifyError } from "@/lib/notify";
import { loadSettings, useSettings } from "@/lib/settings";
import { formatBytes } from "@/lib/storage";

/** Steps report a running move so the sheet can't be left halfway through it. */
export type StepProps = { onBusy: (busy: boolean) => void };

/** The default data folder, or another one: moved like Settings does (nothing to copy yet). */
export function FolderStep({ onBusy }: StepProps) {
  const [settings] = useSettings();
  const [progress, setProgress] = useState<MoveProgress | null>(null);
  const change = async () => {
    const dir = await pickFolder(
      "Choose the data folder",
      "~/mangas",
      parentFolder(settings.dataDir),
    );
    if (!dir || dir === settings.dataDir) return;
    onBusy(true);
    setProgress({ copied: 0, total: 0 });
    let unlisten: (() => void) | undefined;
    try {
      unlisten = await onMoveProgress(setProgress);
      await moveDataDir(dir);
      await loadSettings();
    } catch (e: unknown) {
      notifyError("Couldn't use that folder", e);
    } finally {
      unlisten?.();
      setProgress(null);
      onBusy(false);
    }
  };
  const busy = progress !== null;
  return (
    <div className="flex flex-col gap-2">
      <div className="flex gap-2">
        <Input
          readOnly
          value={settings.dataDir}
          aria-label="Data folder"
          className="font-mono text-caption"
        />
        <Button onClick={change} disabled={busy} className="shrink-0">
          <FolderInput /> {busy ? "Moving…" : "Change…"}
        </Button>
      </div>
      {progress && (
        <>
          <Progress
            value={progress.copied}
            max={progress.total}
            aria-label="Moving the data folder"
          />
          <p className="text-caption text-ink-3">
            Copying {formatBytes(progress.copied)} of {formatBytes(progress.total)}… Keep the app
            open until it finishes.
          </p>
        </>
      )}
    </div>
  );
}

/** The export device profiles; the choice becomes the default device. */
export function ReaderStep() {
  const [settings, update] = useSettings();
  const devices = useAsync(listDevices, []);
  if (devices.state.kind !== "ready") return <DevicesPending state={devices.state} />;
  return (
    <DeviceSelect
      devices={devices.state.data}
      value={settings.defaultDevice}
      onChange={(id) => update({ defaultDevice: id })}
    />
  );
}

export function ProvidersStep() {
  return (
    <div className="max-h-80 overflow-y-auto">
      <ProvidersList />
      <p className="mt-3 text-caption text-ink-3">
        Press <kbd className="font-mono">?</kbd> any time to see the keyboard shortcuts.
      </p>
    </div>
  );
}
