import { useCallback, useState } from "react";
import { toast } from "sonner";
import { type ImportKind, pickImportPaths } from "@/lib/dialogs";
import { importSummary } from "@/lib/import-summary";
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
      toast.loading("Importing…", { id: IMPORT_TOAST, description: null });
      try {
        const report = await importLocal(paths);
        toast.success(importSummary(report), { id: IMPORT_TOAST, description: null });
        onImported(report);
      } catch (e) {
        notifyError("Couldn't import", e, { id: IMPORT_TOAST });
      } finally {
        setBusy(false);
      }
    },
    [onImported],
  );

  return { busy, start };
}
