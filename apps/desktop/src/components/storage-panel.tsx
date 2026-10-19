import { Eraser } from "lucide-react";
import { toast } from "sonner";
import { DataFolderField } from "@/components/data-folder-field";
import { ErrorAlert } from "@/components/error-alert";
import { Field } from "@/components/field";
import { TitleUsageList } from "@/components/title-usage-list";
import { Button } from "@/components/ui/button";
import { UsageBar } from "@/components/usage-bar";
import { useAsync } from "@/hooks/use-async";
import { useAppFocus } from "@/lib/app-focus";
import { clearFailure } from "@/lib/detector-download";
import { type CacheKind, clearCache, storageUsage } from "@/lib/ipc.storage";
import { notifyError } from "@/lib/notify";
import { formatBytes } from "@/lib/storage";
import { storageChanged, useStorageChanged } from "@/lib/storage-changed";

const CACHES: Record<CacheKind, { label: string; done: string; hint: string }> = {
  covers: {
    label: "Clear cover cache",
    done: "Cover cache cleared",
    hint: "fetched again when shown",
  },
  models: {
    label: "Delete detector support",
    done: "Detector support deleted",
    hint: "download it again in Settings → Export or the export dialog",
  },
};

/** Settings → Storage: data folder, what each folder takes, caches and per-title downloads. */
export function StoragePanel() {
  const usage = useAsync(storageUsage, []);
  useAppFocus(usage.reload);
  useStorageChanged(usage.reload);
  const ready = usage.state.kind === "ready" ? usage.state.data : null;

  const clear = async (cache: CacheKind) => {
    try {
      const cleared = await clearCache(cache);
      toast.success(CACHES[cache].done, {
        description: `${formatBytes(cleared.bytes)} freed; ${CACHES[cache].hint}.`,
      });
    } catch (e: unknown) {
      notifyError(`Couldn't ${CACHES[cache].label.toLowerCase()}`, e);
    }
    // As after Remove on the detector card: a failure from before the delete no longer applies.
    if (cache === "models") clearFailure();
    storageChanged();
  };

  return (
    <>
      <DataFolderField onMoved={usage.reload} />
      <Field label="Disk usage">
        {usage.state.kind === "error" && (
          <ErrorAlert
            title="Couldn't measure disk usage"
            message={usage.state.message}
            onRetry={usage.reload}
          />
        )}
        {usage.state.kind === "loading" && <p className="text-caption text-ink-3">Measuring…</p>}
        {ready && <UsageBar usage={ready} />}
      </Field>
      <Field
        label="Caches"
        hint="The app rebuilds these on its own: covers are fetched again when shown, detector support (ONNX Runtime and models) downloads again from Settings → Export or the export dialog."
      >
        <div className="flex flex-wrap gap-2">
          {(Object.keys(CACHES) as CacheKind[]).map((cache) => (
            <Button key={cache} onClick={() => void clear(cache)} disabled={!ready}>
              <Eraser /> {CACHES[cache].label}
              {ready && (
                <span className="text-ink-3 tabular-nums">{formatBytes(ready[cache])}</span>
              )}
            </Button>
          ))}
        </div>
      </Field>
      <Field
        label="Downloads by title"
        hint="Deleting a title's downloads keeps it in your library with its chapter list."
      >
        {ready && <TitleUsageList titles={ready.titles} onChange={usage.reload} />}
      </Field>
    </>
  );
}
