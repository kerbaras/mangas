import { Download, Upload } from "lucide-react";
import { toast } from "sonner";
import { Field } from "@/components/field";
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
        description: `${plural(summary.titles, "title")} in ${path}.${local}`,
      });
    } catch (e: unknown) {
      notifyError("Couldn't save the backup", e);
    }
  };

  const restore = async () => {
    const path = await pickBackupFile();
    if (!path) return;
    try {
      const restored = await importBackup(path);
      if (restored.settings) await loadSettings();
      toast.success("Backup imported", { description: restoreSummary(restored) });
    } catch (e: unknown) {
      notifyError("Couldn't import the backup", e);
    }
  };

  return (
    <Field
      label="Library backup"
      hint="Followed titles and preferences in one JSON file. Provider sessions and the data folder aren't included. Import adds titles you don't follow yet and also reads HaruNeko bookmark exports."
    >
      <div className="flex flex-wrap gap-2">
        <Button onClick={() => void save()}>
          <Download /> Export backup…
        </Button>
        <Button onClick={() => void restore()}>
          <Upload /> Import backup…
        </Button>
      </div>
    </Field>
  );
}
