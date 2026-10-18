import { Send } from "lucide-react";
import { useState } from "react";
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
import { type Async, useAsync } from "@/hooks/use-async";
import { useExport } from "@/hooks/use-export";
import { useExportPlan } from "@/hooks/use-export-plan";
import { useExportTarget } from "@/hooks/use-export-target";
import { type Book, useSend } from "@/hooks/use-send";
import { exportJob } from "@/lib/export-job";
import type { ExportPick } from "@/lib/export-name";
import {
  type BundleMode,
  detectorsDefault,
  detectorsUsable,
  exportAction,
  initialMode,
  pickSummary,
  withExtension,
} from "@/lib/export-plan";
import { detectorsStatus, type ExportResult, type Preset } from "@/lib/ipc";
import type { VolumePlan } from "@/lib/ipc.volumes";
import { titleOf } from "@/lib/send-labels";
import { useSettings } from "@/lib/settings";

type Props = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Downloaded chapters to export, grouped by series. */
  picks: ExportPick[];
  /** Called once the books are written, so lists can show them as exported. */
  onExported?: (results: ExportResult[]) => void;
};

/** The one export flow: what to write (bundle, title), how (profile, format, smart cut), where. */
export function ExportDialog({ open, onOpenChange, picks, onExported }: Props) {
  const [settings] = useSettings();
  const chapters = picks.flatMap((p) => p.chapters);
  const [mode, setMode] = useState<BundleMode>(() =>
    picks.length > 1 ? "single" : initialMode(chapters),
  );
  const [size, setSize] = useState(10);
  const [title, setTitle] = useState("");
  const [preset, setPreset] = useState<Preset>(settings.defaultPreset);
  const [detectorsOn, setDetectorsOn] = useState<boolean | null>(null);
  const [delivery, setDelivery] = useState<Delivery>("save");
  const [written, setWritten] = useState<Book[]>([]);
  const folder = useOutputFolder();
  const target = useExportTarget();
  const detectors = useAsync(detectorsStatus, []);
  const job = useExport();
  const send = useSend();
  const { plans, fresh, joined } = useExportPlan({ open, picks, mode, size, title });
  const emailBlocked = useEmailBlock(target.format) !== null;
  const via = delivery === "email" && emailBlocked ? "save" : delivery;
  const useDetectors =
    detectorsUsable(detectors.state) && (detectorsOn ?? detectorsDefault(detectors.state));
  const files: Async<VolumePlan[]> =
    plans.kind === "ready"
      ? {
          kind: "ready",
          data: plans.data.map((p) => ({
            ...p,
            file_name: withExtension(p.file_name, target.format),
          })),
        }
      : plans;
  const books = files.kind === "ready" ? files.data.length : 0;
  const sending = ["looking", "copying", "emailing"].includes(send.state.kind);
  const busy = job.state.kind === "running" || sending;
  const ready =
    fresh && books > 0 && folder.output.trim() !== "" && target.devices.kind === "ready";

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
        outputDir: folder.output,
      }),
    );
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
              <VolumePreview plans={files} folder={folder.output} stale={!fresh} />
            </Field>
          </div>
          <ExportOptions
            target={target}
            preset={preset}
            onPreset={setPreset}
            detectorsStatus={detectors.state}
            detectors={useDetectors}
            onDetectors={setDetectorsOn}
            output={{ value: folder.output, fallback: folder.fallback, onChange: folder.setOutput }}
            delivery={via}
            onDelivery={setDelivery}
          />
        </div>
        <div className="mt-4 flex flex-col gap-3 empty:hidden">
          <ExportProgressPanel state={job.state} onReset={job.reset} />
          <SendStatus
            state={send.state}
            onRetry={() => void send.deliver(written, via === "email" ? "email" : "usb")}
          />
        </div>
        <DialogFooter>
          <Button disabled={busy} onClick={() => onOpenChange(false)}>
            {job.state.kind === "done" ? "Close" : "Cancel"}
          </Button>
          <Button variant="primary" disabled={busy || !ready} onClick={() => void run()}>
            <Send /> {exportAction(books, via)}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
