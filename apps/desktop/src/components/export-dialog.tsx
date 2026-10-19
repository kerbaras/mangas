import { Send } from "lucide-react";
import { useCallback, useState } from "react";
import { BundleField } from "@/components/bundle-field";
import type { Delivery } from "@/components/delivery-picker";
import { useEmailBlock } from "@/components/delivery-picker";
import { ExportOptions } from "@/components/export-options";
import { ExportProgressPanel } from "@/components/export-progress";
import { Field } from "@/components/field";
import { useOutputFolder } from "@/components/output-folder";
import { SendStatus } from "@/components/send-status";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { VolumePreview } from "@/components/volume-preview";
import { useClearOnChange } from "@/hooks/use-clear-on-change";
import { useDetectorSupport } from "@/hooks/use-detector-support";
import { useExport } from "@/hooks/use-export";
import { useExportPlan } from "@/hooks/use-export-plan";
import { useFreeNames } from "@/hooks/use-free-names";
import { type Book, runningVia, useSend } from "@/hooks/use-send";
import { useSeriesTarget } from "@/hooks/use-series-target";
import { exportJob } from "@/lib/export-job";
import type { ExportPick } from "@/lib/export-name";
import {
  type BundleMode,
  detectorsDefault,
  detectorsUsable,
  exportAction,
  inFormat,
  initialMode,
  pickSummary,
} from "@/lib/export-plan";
import type { ExportResult } from "@/lib/ipc";
import { titleOf } from "@/lib/send-labels";

type Props = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Downloaded chapters to export, grouped by series. */
  picks: ExportPick[];
  /** Called once the books are written, so lists can show them as exported. */
  onExported?: (results: ExportResult[]) => void;
  /** Called once a delivery sent at least one book, so the history can show it. */
  onSent?: () => void;
};

/** The one export flow: what to write (bundle, title), how (profile, format, smart cut), where. */
export function ExportDialog({ open, onOpenChange, picks, onExported, onSent }: Props) {
  const chapters = picks.flatMap((p) => p.chapters);
  const [mode, setMode] = useState<BundleMode>(() =>
    picks.length > 1 ? "single" : initialMode(chapters),
  );
  const [size, setSize] = useState(10);
  const [title, setTitle] = useState("");
  const [detectorsOn, setDetectorsOn] = useState<boolean | null>(null);
  const [delivery, setDelivery] = useState<Delivery>("save");
  const [written, setWritten] = useState<Book[]>([]);
  const [runs, setRuns] = useState(0);
  const folder = useOutputFolder();
  const { target, preset, setPreset, cut, settled } = useSeriesTarget(picks);
  const detectors = useDetectorSupport();
  const job = useExport();
  const send = useSend(onSent);
  const { plans, fresh, joined } = useExportPlan({ open, picks, mode, size, title });
  const emailBlocked = useEmailBlock(target.format) !== null;
  const via = delivery === "email" && emailBlocked ? "save" : delivery;
  const useDetectors =
    detectorsUsable(detectors.status) && (detectorsOn ?? detectorsDefault(detectors.status));
  const planned = inFormat(plans, target.format);
  const { files, named } = useFreeNames(planned, folder.resolved, runs, target.replace);
  const books = files.kind === "ready" ? files.data.length : 0;
  const sending = runningVia(send.state);
  const busy = job.state.kind === "running" || sending !== undefined;
  const failed = written.length > 0 && ["error", "no-device"].includes(send.state.kind);
  const sendVia = via === "save" ? undefined : via;
  const resend = failed && sendVia !== undefined;
  const ready =
    fresh &&
    named &&
    books > 0 &&
    (folder.picked === null || folder.picked.trim() !== "") &&
    target.devices.kind === "ready" &&
    detectors.status.kind !== "loading" &&
    settled;
  const [resetJob, resetSend] = [job.reset, send.reset];
  const clearResult = useCallback(() => {
    resetJob();
    resetSend();
    setWritten([]);
  }, [resetJob, resetSend]);
  const choice = [
    mode,
    size,
    title,
    target.device,
    target.format,
    preset,
    useDetectors,
    folder.output,
    target.replace,
  ].join("|");
  useClearOnChange(choice, busy, clearResult);
  const keepFailed = useCallback(() => !failed && clearResult(), [failed, clearResult]);
  useClearOnChange(via, busy, keepFailed);

  const run = async () => {
    send.reset();
    const results = await job.run(
      exportJob({
        picks,
        mode,
        size,
        title,
        device: target.device,
        format: target.format,
        preset,
        detectors: useDetectors,
        outputDir: folder.picked,
        replace: target.replace,
      }),
    );
    setRuns((n) => n + 1);
    if (!results) return;
    onExported?.(results);
    const sent = results.map((r) => ({ file: r.output, title: titleOf(r.output) }));
    setWritten(sent);
    if (via !== "save") await send.deliver(sent, via);
  };

  return (
    <Dialog open={open} onOpenChange={(o) => !busy && onOpenChange(o)}>
      <DialogContent className="max-h-[90vh] max-w-4xl overflow-y-auto">
        <DialogTitle>Export</DialogTitle>
        <DialogDescription>
          {pickSummary(picks)}.{" "}
          {joined
            ? "Chapters from several series go into one book."
            : "Each book gets the series cover, a table of contents and the series details."}
        </DialogDescription>
        <div className="mt-4 grid grid-cols-2 gap-6">
          <div className="flex min-w-0 flex-col gap-4">
            {!joined && <BundleField mode={mode} onMode={setMode} size={size} onSize={setSize} />}
            {(mode === "single" || books === 1) && (
              <Field label="Title" htmlFor="export-title" hint="Also the file name.">
                <Input
                  id="export-title"
                  value={title}
                  onChange={(e) => setTitle(e.target.value)}
                  placeholder={plans.kind === "ready" ? (plans.data[0]?.title ?? "") : ""}
                />
              </Field>
            )}
            <Field label={books > 1 ? `${books} files` : "File"}>
              <VolumePreview plans={files} folder={folder.resolved} stale={!fresh || !named} />
            </Field>
          </div>
          <ExportOptions
            target={target}
            preset={preset}
            onPreset={setPreset}
            detectorSupport={detectors}
            detectors={useDetectors}
            onDetectors={setDetectorsOn}
            output={{ value: folder.output, fallback: folder.fallback, onChange: folder.setOutput }}
            delivery={via}
            onDelivery={setDelivery}
            cut={{ ...cut, chapters }}
          />
        </div>
        <div className="mt-4 flex flex-col gap-3 empty:hidden">
          <ExportProgressPanel state={job.state} onReset={job.reset} />
          <SendStatus
            state={send.state}
            via={via}
            onRetry={sendVia && (() => void send.retry(sendVia))}
          />
        </div>
        <DialogFooter>
          <Button disabled={busy} onClick={() => onOpenChange(false)}>
            {job.state.kind === "done" ? "Close" : "Cancel"}
          </Button>
          <Button
            variant="primary"
            disabled={busy || !ready}
            onClick={() => void (resend ? send.retry(sendVia) : run())}
          >
            <Send /> {exportAction(books, via, resend, sending)}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
