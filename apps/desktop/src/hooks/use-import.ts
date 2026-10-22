import { useCallback, useState } from "react";
import { toast } from "sonner";
import { tr } from "@/i18n/tr";
import { type ImportKind, pickImportPaths } from "@/lib/dialogs";
import { notifyImported } from "@/lib/import-toast";
import { type ImportReport, importLocal } from "@/lib/ipc";
import { notifyError } from "@/lib/notify";

const IMPORT_TOAST = "import";

/** Picks paths with the native dialog and imports them, reporting progress and the result in one toast. */
export function useImport(onImported: (report: ImportReport) => void) {
  const [busy, setBusy] = useState(false);

  const start = useCallback(
    async (kind: ImportKind) => {
      const paths = await pickImportPaths(kind);
      if (paths.length === 0) return;
      setBusy(true);
      toast.loading(tr("library.importing"), { id: IMPORT_TOAST, description: null });
      try {
        const report = await importLocal(paths);
        notifyImported(report, IMPORT_TOAST);
        onImported(report);
      } catch (e) {
        notifyError(tr("import.failed"), e, { id: IMPORT_TOAST });
      } finally {
        setBusy(false);
      }
    },
    [onImported],
  );

  return { busy, start };
}
