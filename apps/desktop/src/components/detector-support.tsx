import { Download, Trash2, X } from "lucide-react";
import { toast } from "sonner";
import { ErrorAlert } from "@/components/error-alert";
import { Button } from "@/components/ui/button";
import { Progress } from "@/components/ui/progress";
import type { DetectorSupport } from "@/hooks/use-detector-support";
import { downloadLabel, installedLabel, progressLabel, runtimeFailure } from "@/lib/detector-hint";
import { notifyError } from "@/lib/notify";
import { formatBytes } from "@/lib/storage";

type Props = {
  support: DetectorSupport;
  /** Settings shows what is installed and a remove action; the export dialog doesn't. */
  manage?: boolean;
};

/**
 * Detector support download block: the download action, its progress with Cancel, a failure
 * with Retry, and (with `manage`) what is installed and Remove. Nothing for builds without it.
 */
export function DetectorSupportActions({ support, manage = false }: Props) {
  const status = support.status.kind === "ready" ? support.status.data : null;
  if (!status?.available) return null;
  if (support.downloading) {
    const p = support.progress;
    return (
      <div className="flex flex-col gap-1.5">
        <Progress value={p?.done ?? 0} max={p?.total ?? 0} />
        <div className="flex items-center justify-between gap-2">
          <span className="text-caption text-ink-3 tabular-nums">{progressLabel(p)}</span>
          <Button size="sm" variant="ghost" onClick={support.cancel}>
            <X /> Cancel
          </Button>
        </div>
      </div>
    );
  }
  const failure = support.failure ?? runtimeFailure(status);
  if (failure)
    return (
      <ErrorAlert
        title="Couldn't set up detector support"
        message={`${failure}. Exports still work without detectors.`}
        onRetry={() => void support.download()}
      />
    );
  if (!status.downloaded)
    return (
      <div>
        <Button size="sm" onClick={() => void support.download()}>
          <Download /> {downloadLabel(status.downloadBytes)}
        </Button>
      </div>
    );
  if (!manage) return null;
  const remove = async () => {
    try {
      const freed = await support.remove();
      toast.success("Detector support removed", {
        description: `${formatBytes(freed)} freed; download it again here or in the export dialog.`,
      });
    } catch (e: unknown) {
      notifyError("Couldn't remove detector support", e);
    }
  };
  return (
    <div className="flex flex-wrap items-center justify-between gap-2">
      <span className="text-caption text-ink-3 tabular-nums">{installedLabel(status)}</span>
      <Button size="sm" onClick={() => void remove()}>
        <Trash2 /> Remove detector support
      </Button>
    </div>
  );
}
