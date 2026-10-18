import { RotateCcw, Save, Scissors } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { CutPreviewPane, type PreviewState } from "@/components/cut-preview-pane";
import { SeriesPresetHint } from "@/components/series-preset-hint";
import { SmartCutForm } from "@/components/smart-cut-form";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useDebounced } from "@/hooks/use-debounced";
import type { Series } from "@/hooks/use-series-preset";

const errorMessage = (e: unknown) => (e instanceof Error ? e.message : String(e));

import type { Device, Preset } from "@/lib/ipc";
import {
  cancelCutPreview,
  clearExportPreset,
  type ExportPreset,
  previewCut,
  setExportPreset,
} from "@/lib/ipc.cut-preview";
import type { ExportFormat } from "@/lib/ipc.export";

export type PreviewChapter = { id: string; label: string; dir: string };

type Props = {
  series: Series | null;
  chapters: PreviewChapter[];
  device: Device | undefined;
  format: ExportFormat;
  cutPreset: Preset;
  detectors: boolean;
  saved: ExportPreset | null;
  onSaved: () => void;
};

const cutOptions = ({ format: _f, profile: _p, ...rest }: ExportPreset): ExportPreset => rest;

/** The one entry to the smart-cut options: a button, the "Using series preset" hint and the dialog. */
export function SmartCutButton(props: Props) {
  const [open, setOpen] = useState(false);
  const usable = props.series !== null && props.chapters.length > 0 && props.device;
  return (
    <div className="flex items-center justify-between gap-4">
      {props.saved ? (
        <SeriesPresetHint preset={props.saved} />
      ) : (
        <p className="text-caption text-ink-3">
          {usable
            ? "Preview the cut and save options for this series."
            : "Pick chapters of one series to preview the cut."}
        </p>
      )}
      <Button size="sm" disabled={!usable} onClick={() => setOpen(true)}>
        <Scissors /> Smart-cut options…
      </Button>
      {open && usable && <SmartCutPreview {...props} onClose={() => setOpen(false)} />}
    </div>
  );
}

function SmartCutPreview(props: Props & { onClose: () => void }) {
  const { series, chapters, device, format, cutPreset, detectors, saved } = props;
  const [form, setForm] = useState<ExportPreset>(() => cutOptions(saved ?? {}));
  const [chapter, setChapter] = useState(chapters[0]?.id);
  const [state, setState] = useState<PreviewState>({ kind: "loading" });
  const [note, setNote] = useState<string | null>(null);
  const options = useDebounced(form, 400);
  const dir = chapters.find((c) => c.id === chapter)?.dir;

  // Closing the panel stops its previews in the backend, even one whose request arrives later.
  // Declared first so a (re)mount gets its id before the preview effect runs.
  const panel = useRef("");
  useEffect(() => {
    const id = crypto.randomUUID();
    panel.current = id;
    return () => void cancelCutPreview(id).catch(() => {});
  }, []);

  useEffect(() => {
    if (!dir || !device) return;
    let live = true;
    setState({ kind: "loading" });
    const request = { cut_preset: cutPreset, ...options, profile: device.id };
    previewCut(dir, request, detectors, panel.current).then(
      (preview) => live && setState({ kind: "ready", preview }),
      (e) => live && setState({ kind: "error", message: errorMessage(e) }),
    );
    return () => {
      live = false;
    };
  }, [dir, options, device, detectors, cutPreset]);

  const save = async () => {
    if (!series || !device) return;
    try {
      await setExportPreset(series.source, series.id, {
        cut_preset: cutPreset,
        ...form,
        format,
        profile: device.id,
      });
      setNote("Saved cut settings, device and format for this series.");
      props.onSaved();
    } catch (e) {
      setNote(`Couldn't save: ${errorMessage(e)}`);
    }
  };

  const reset = async () => {
    setForm({});
    if (!series || !saved) return;
    try {
      await clearExportPreset(series.source, series.id);
      setNote("Series preset removed: exports use the defaults.");
      props.onSaved();
    } catch (e) {
      setNote(`Couldn't remove the preset: ${errorMessage(e)}`);
    }
  };

  return (
    <Dialog open onOpenChange={(o) => !o && props.onClose()}>
      <DialogContent className="max-w-5xl">
        <DialogTitle>Smart-cut options</DialogTitle>
        <DialogDescription>
          {series?.title}: pages as {device?.name} will show them. Saving keeps these cut settings,
          the device and the format for every export of this series.
        </DialogDescription>
        <div className="grid min-h-0 grid-cols-[280px_1fr] gap-5">
          <div className="flex max-h-[60vh] flex-col gap-4 overflow-auto pr-1">
            <Select value={chapter} onValueChange={setChapter}>
              <SelectTrigger aria-label="Preview chapter">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {chapters.map((c) => (
                  <SelectItem key={c.id} value={c.id}>
                    {c.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            <SmartCutForm
              value={form}
              onChange={setForm}
              basePreset={cutPreset}
              grayscale={device?.grayscale ?? true}
            />
          </div>
          <CutPreviewPane state={state} />
        </div>
        <DialogFooter className="items-center">
          {note && <span className="mr-auto text-caption text-ink-3">{note}</span>}
          <Button onClick={reset}>
            <RotateCcw /> Reset to defaults
          </Button>
          <Button variant="primary" onClick={save}>
            <Save /> Save cut, device and format for this series
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
