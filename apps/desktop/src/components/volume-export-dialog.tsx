import { FolderOpen, Send } from "lucide-react";
import { useState } from "react";
import { DeviceSelect } from "@/components/device-select";
import { DevicesPending } from "@/components/devices-pending";
import { ExportProgressPanel } from "@/components/export-progress";
import { Field } from "@/components/field";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { VolumePreview } from "@/components/volume-preview";
import { type Async, useAsync } from "@/hooks/use-async";
import { useDebounced } from "@/hooks/use-debounced";
import { useVolumeExport } from "@/hooks/use-volume-export";
import { revealFolder } from "@/lib/dialogs";
import { type Chapter, detectorsStatus, listDevices, type Manga } from "@/lib/ipc";
import { type Grouping, planVolumes, type VolumePlan } from "@/lib/ipc.volumes";
import { useSettings } from "@/lib/settings";

type Props = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  manga: Pick<Manga, "source" | "id" | "title">;
  /** Downloaded chapters to bundle, any order. */
  chapters: Chapter[];
  /** Called once the books are written, so the chapter list can show them as exported. */
  onExported?: () => void;
};

type Mode = Grouping["kind"];

const volumes = (chapters: Chapter[]) => new Set(chapters.map((c) => c.volume?.trim() || null));

/** Opens on "By volume" when the chapters span several provider volumes, else one book. */
export const initialMode = (chapters: Chapter[]): Mode =>
  volumes(chapters).size > 1 ? "by_volume" : "single";

/** Typing in the title or size fields replans once the user pauses. */
const PLAN_DELAY_MS = 300;

type Keyed = { key: string; data: VolumePlan[] };

/** The last plans that arrived; they stay on screen while a newer request is in flight. */
const settled = (state: Async<Keyed>): Async<VolumePlan[]> =>
  state.kind === "ready" ? { kind: "ready", data: state.data.data } : state;

export function VolumeExportDialog({ open, onOpenChange, manga, chapters, onExported }: Props) {
  const [settings] = useSettings();
  const [mode, setMode] = useState<Mode>(() => initialMode(chapters));
  const [size, setSize] = useState(10);
  const [title, setTitle] = useState("");
  const [device, setDevice] = useState(settings.defaultDevice);
  const devices = useAsync(listDevices, []);
  const detectors = useAsync(detectorsStatus, []);
  const job = useVolumeExport();
  const outputDir = `${settings.dataDir}/kindle`;
  const planSize = useDebounced(size, PLAN_DELAY_MS);
  const planTitle = useDebounced(title, PLAN_DELAY_MS);
  const ids = chapters.map((c) => c.id);
  const base = { source: manga.source, mangaId: manga.id, chapterIds: ids };
  const grouping = (n: number): Grouping =>
    mode === "every" ? { kind: "every", size: n } : { kind: mode };
  const keyOf = (n: number, t: string) => [open, ids.join(), mode, n, t].join("|");
  const key = keyOf(planSize, planTitle);
  const planned = useAsync(
    () =>
      planVolumes({ ...base, grouping: grouping(planSize), title: planTitle }).then((data) => ({
        key,
        data,
      })),
    [key],
  );
  const fresh = planned.state.kind === "ready" && planned.state.data.key === keyOf(size, title);
  const plans = settled(planned.state);
  const books = plans.kind === "ready" ? plans.data.length : 0;
  const busy = job.state.kind === "running";
  const useDetectors =
    detectors.state.kind === "ready" &&
    detectors.state.data.available &&
    detectors.state.data.downloaded;

  const run = async () => {
    const written = await job.run({
      ...base,
      grouping: grouping(size),
      title,
      device,
      preset: settings.defaultPreset,
      detectors: useDetectors,
      outputDir,
    });
    if (written) onExported?.();
  };

  return (
    <Dialog open={open} onOpenChange={(o) => !busy && onOpenChange(o)}>
      <DialogContent className="max-w-xl">
        <DialogTitle>Export as volume</DialogTitle>
        <DialogDescription>
          {chapters.length} downloaded {chapters.length === 1 ? "chapter" : "chapters"}. Each book
          gets the series cover, a table of contents with one entry per chapter, and the series and
          author details.
        </DialogDescription>
        <div className="mt-4 flex flex-col gap-4">
          <Field label="Bundle">
            <div className="flex items-center gap-3">
              <Tabs value={mode} onValueChange={(v) => setMode(v as Mode)}>
                <TabsList aria-label="Bundle">
                  <TabsTrigger value="single">One book</TabsTrigger>
                  <TabsTrigger value="by_volume">By volume</TabsTrigger>
                  <TabsTrigger value="every">Every N chapters</TabsTrigger>
                </TabsList>
              </Tabs>
              {mode === "every" && (
                <Input
                  type="number"
                  min={1}
                  value={size}
                  onChange={(e) => setSize(Math.max(1, Number(e.target.value) || 1))}
                  aria-label="Chapters per book"
                  className="w-20"
                />
              )}
            </div>
          </Field>
          {(mode === "single" || books === 1) && (
            <Field label="Title" htmlFor="volume-title" hint="Also the file name.">
              <Input
                id="volume-title"
                value={title}
                onChange={(e) => setTitle(e.target.value)}
                placeholder={plans.kind === "ready" ? (plans.data[0]?.title ?? "") : ""}
              />
            </Field>
          )}
          <Field label="Device" htmlFor="volume-device">
            {devices.state.kind === "ready" ? (
              <DeviceSelect
                id="volume-device"
                devices={devices.state.data}
                value={device}
                onChange={setDevice}
              />
            ) : (
              <DevicesPending state={devices.state} />
            )}
          </Field>
          <Field label={books > 1 ? `${books} files` : "File"}>
            <VolumePreview plans={plans} folder={outputDir} stale={!fresh} />
          </Field>
          {job.state.kind === "done" ? (
            <div
              className="flex items-center justify-between gap-3 rounded-md border border-stroke bg-surface p-3"
              role="status"
            >
              <p className="text-body font-medium">
                Wrote {job.state.results.length} {job.state.results.length === 1 ? "book" : "books"}
              </p>
              <Button size="sm" onClick={() => void revealFolder(outputDir)}>
                <FolderOpen /> Show folder
              </Button>
            </div>
          ) : (
            job.state.kind !== "idle" && (
              <ExportProgressPanel state={job.state} onReset={job.reset} />
            )
          )}
        </div>
        <DialogFooter>
          <Button disabled={busy} onClick={() => onOpenChange(false)}>
            {job.state.kind === "done" ? "Close" : "Cancel"}
          </Button>
          <Button
            variant="primary"
            disabled={busy || !fresh || books === 0}
            onClick={() => void run()}
          >
            <Send /> Export {books > 1 ? `${books} books` : "book"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
