import { Send } from "lucide-react";
import { useCallback, useState } from "react";
import { BundleField } from "@/components/bundle-field";
import { type Delivery, useEmailBlock } from "@/components/delivery-picker";
import { ExportOptions } from "@/components/export-options";
import { ExportProgressPanel } from "@/components/export-progress";
import { ExportTitleField } from "@/components/export-title-field";
import { Field } from "@/components/field";
import { useOutputFolder } from "@/components/output-folder";
import { OversizedAlert } from "@/components/oversized-alert";
import { SendStatus } from "@/components/send-status";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogTitle,
} from "@/components/ui/dialog";
import { VolumePreview } from "@/components/volume-preview";
import { useBundle } from "@/hooks/use-bundle";
import { useClearOnChange } from "@/hooks/use-clear-on-change";
import { useDetectorSupport } from "@/hooks/use-detector-support";
import { useExport } from "@/hooks/use-export";
import { useExportPlan } from "@/hooks/use-export-plan";
import { useFreeNames } from "@/hooks/use-free-names";
import { useOversized } from "@/hooks/use-oversized";
import { type Book, refused, runningVia, stopped, useSend } from "@/hooks/use-send";
import { useSeriesTarget } from "@/hooks/use-series-target";
import { useSizeEstimate } from "@/hooks/use-size-estimate";
import { booksToSend, exportJob, keptBooks } from "@/lib/export-job";
import type { ExportPick } from "@/lib/export-name";
import {
  detectorsDefault,
  detectorsUsable,
  exportAction,
  inFormat,
  pickSummary,
} from "@/lib/export-plan";
import type { ExportResult } from "@/lib/ipc";
import { oversizedOf, PART_MB, WEB_PART_MB } from "@/lib/kindle-size";

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
  const { mode, setMode, size, setSize, every } = useBundle(picks);
  const [title, setTitle] = useState("");
  const [detectorsOn, setDetectorsOn] = useState<boolean | null>(null);
  const [delivery, setDelivery] = useState<Delivery>("save");
  const [splitMb, setSplitMb] = useState<number | null>(null);
  const [written, setWritten] = useState<Book[]>([]);
  const folder = useOutputFolder();
  const { target, preset, setPreset, cut, settled } = useSeriesTarget(picks);
  const detectors = useDetectorSupport();
  const job = useExport({ visible: () => open, folder: folder.resolved });
  const send = useSend(onSent);
  const { plans, fresh, joined } = useExportPlan({ open, picks, mode, size, title });
  const emailBlocked = useEmailBlock(target.format) !== null;
  const via = delivery === "email" && emailBlocked ? "save" : delivery;
  const useDetectors =
    detectorsUsable(detectors.status) && (detectorsOn ?? detectorsDefault(detectors.status));
  const planned = inFormat(plans, target.format);
  const { device, format, replace, quality, direction } = target;
  const [epoch, kept, dir] = [job.settled, keptBooks(job.retry.kept), folder.resolved];
  const { files, named, blocked } = useFreeNames(planned, dir, epoch, replace, splitMb, kept);
  const preview = { folder: folder.resolved, stale: !fresh || !named, blocked };
  const books = files.kind === "ready" ? files.data.length : 0;
  const email = via === "email";
  const budgets = useSizeEstimate({ plans: files, device, preset, quality, email, splitMb });
  const fit = { device, preset, quality, email, splitMb, bundle: !joined };
  const over = useOversized(job.state.kind === "done" ? job.state.results : [], files, fit);
  const sending = runningVia(send.state);
  const busy = job.state.kind === "running" || sending !== undefined;
  const failed = written.length > 0 && stopped(send.state);
  const sendVia = via === "save" ? undefined : via;
  const stuck = failed && refused(send.state, via);
  const resend =
    failed && sendVia !== undefined && !stuck && !(via === "email" && over.files.length > 0);
  const retrySend = sendVia && (() => void send.retry(sendVia));
  const ready =
    fresh &&
    named &&
    !blocked &&
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
  const picked = [mode, size, title, preset, useDetectors, folder.output, splitMb];
  const choice = [...picked, device, format, replace, quality, direction].join("|");
  useClearOnChange(choice, busy, clearResult);
  const keepFailed = useCallback(() => !failed && clearResult(), [failed, clearResult]);
  useClearOnChange(via, busy, keepFailed, true);

  const run = async () => {
    if (!job.retry.kept.length) send.reset();
    if (via === "usb" && !(await send.admits(target.format))) return;
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
        quality: target.quality,
        splitMb,
        direction,
        kept: job.retry.kept,
      }),
    );
    if (!results) return;
    onExported?.(results);
    const sent = booksToSend(send.unsent(), job.retry.kept, results);
    setWritten(sent);
    // A book over the email limit on disk is never emailed: the alert offers the split.
    if (via === "email" && oversizedOf(results, true).length > 0) return;
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
              <ExportTitleField value={title} onChange={setTitle} plans={plans} />
            )}
            <Field label={books > 1 ? `${books} files` : "File"}>
              <VolumePreview plans={files} budgets={budgets} {...preview} />
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
            size={{ budgets, splitMb, onSplit: setSplitMb, onEvery: joined ? undefined : every }}
            delivery={via}
            onDelivery={setDelivery}
            cut={{ ...cut, chapters }}
          />
        </div>
        <div className="mt-4 flex flex-col gap-3 empty:hidden">
          <ExportProgressPanel state={job.state} onRetry={() => void run()} canRetry={ready} />
          <OversizedAlert
            files={over.files}
            email={email}
            levers={over.levers}
            onSplit={splitMb === null ? () => setSplitMb(email ? PART_MB : WEB_PART_MB) : undefined}
          />
          <SendStatus state={send.state} via={via} onRetry={retrySend} />
        </div>
        <DialogFooter>
          <Button disabled={busy} onClick={() => onOpenChange(false)}>
            {job.state.kind === "done" ? "Close" : "Cancel"}
          </Button>
          <Button
            variant="primary"
            disabled={busy || !ready || stuck}
            onClick={() => void (resend ? send.retry(sendVia) : run())}
          >
            <Send /> {exportAction(books, via, resend, sending)}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
