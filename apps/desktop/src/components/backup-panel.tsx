import { Download, Upload } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";
import { Field } from "@/components/field";
import { RestoreDialog } from "@/components/restore-dialog";
import { Button } from "@/components/ui/button";
import { pickBackupFile, pickBackupTarget } from "@/lib/dialogs";
import { exportBackup, importBackup } from "@/lib/ipc.storage";
import { notifyError } from "@/lib/notify";
import { loadSettings } from "@/lib/settings";
import { plural, restoreSummary } from "@/lib/storage";

/** Settings → Backup: export the library to a JSON file, or merge one back in. */
export function BackupPanel() {
  const save = async () => {
    const path = await pickBackupTarget();
    if (!path) return;
    try {
      const summary = await exportBackup(path);
      const local = summary.skipped_local
        ? ` ${plural(summary.skipped_local, "imported title was", "imported titles were")} left out: their files aren't part of a backup.`
        : "";
      toast.success("Backup saved", {
        id: `backup-save-${path}`,
        action: null,
        description: `${plural(summary.titles, "title")} in ${path}.${local}`,
      });
    } catch (e: unknown) {
      notifyError("Couldn't save the backup", e, { id: `backup-save-${path}` });
    }
  };

  const [pending, setPending] = useState<string | null>(null);
  const pick = async () => setPending(await pickBackupFile());

  const restore = async (path: string, keep: boolean) => {
    setPending(null);
    try {
      const restored = await importBackup(path, keep ? "keep" : "backup");
      await loadSettings();
      toast.success("Backup imported", {
        id: "backup-import",
        action: null,
        description: restoreSummary(restored),
      });
    } catch (e: unknown) {
      notifyError("Couldn't import the backup", e, { id: "backup-import" });
    }
  };

  return (
    <Field
      label="Library backup"
      hint="Followed titles with their categories, reading status and read marks, plus preferences, in one JSON file. Downloaded files, provider sessions and the data folder aren't included. Import adds the titles you don't follow yet and the categories and read marks you're missing, without changing what's already here. It also reads HaruNeko bookmark exports."
    >
      <div className="flex flex-wrap gap-2">
        <Button onClick={() => void save()}>
          <Download /> Export backup…
        </Button>
        <Button onClick={() => void pick()}>
          <Upload /> Import backup…
        </Button>
      </div>
      <RestoreDialog
        path={pending}
        onImport={(keep) => pending && void restore(pending, keep)}
        onCancel={() => setPending(null)}
      />
    </Field>
  );
}
