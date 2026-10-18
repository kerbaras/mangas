import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogTitle,
} from "@/components/ui/dialog";
import { listLibrary } from "@/lib/ipc";

type Props = {
  /** The backup to import; the dialog is open while set. */
  path: string | null;
  onImport: (keepPreferences: boolean) => void;
  onCancel: () => void;
};

/**
 * Asks whose preferences to end up with before importing a backup. Keeping yours is the
 * default when the library already has titles (merging a backup in); an empty library (a new
 * machine) takes the backup's.
 */
export function RestoreDialog({ path, onImport, onCancel }: Props) {
  const [keep, setKeep] = useState(true);
  /** Import waits for the library check that picks the default. */
  const [ready, setReady] = useState(false);
  useEffect(() => {
    if (!path) return;
    let alive = true;
    setReady(false);
    listLibrary()
      .then((titles) => alive && setKeep(titles.length > 0))
      .catch(() => {})
      .finally(() => alive && setReady(true));
    return () => {
      alive = false;
    };
  }, [path]);

  return (
    <Dialog open={path !== null} onOpenChange={(o) => !o && onCancel()}>
      <DialogContent>
        <DialogTitle>Import backup</DialogTitle>
        <DialogDescription>
          Adds the titles, categories and read marks from {path} that aren&apos;t in your library
          yet. Nothing already here is removed.
        </DialogDescription>
        <label htmlFor="keep-preferences" className="flex items-start gap-2 text-body">
          <Checkbox
            id="keep-preferences"
            className="mt-0.5"
            checked={keep}
            onChange={(e) => setKeep(e.target.checked)}
          />
          <span className="flex flex-col gap-0.5">
            <span className="font-medium">Keep my current preferences</span>
            <span className="text-caption text-ink-3">
              Off: languages, providers, export and update settings come from the backup. Provider
              sessions and the data folder are never replaced.
            </span>
          </span>
        </label>
        <DialogFooter>
          <DialogClose asChild>
            <Button>Cancel</Button>
          </DialogClose>
          <Button variant="primary" disabled={!ready} onClick={() => onImport(keep)}>
            Import
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
