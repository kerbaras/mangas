import { FolderOpen, Send } from "lucide-react";
import { useEffect, useState } from "react";
import { DeviceSelect } from "@/components/device-select";
import { DownloadedPicker } from "@/components/downloaded-picker";
import { EmptyState } from "@/components/empty-state";
import { ExportProgressPanel } from "@/components/export-progress";
import { Field } from "@/components/field";
import { PresetPicker } from "@/components/preset-picker";
import { ScreenHeader } from "@/components/screen-header";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import { useAsync } from "@/hooks/use-async";
import { useDownloadedChapters } from "@/hooks/use-downloaded-chapters";
import { useExport } from "@/hooks/use-export";
import { pickFolder } from "@/lib/dialogs";
import { exportFileName, exportTitle, joinPath } from "@/lib/export-name";
import { detectorsStatus, listDevices, type Preset } from "@/lib/ipc";
import { useNavigation } from "@/lib/navigation";
import { useSettings } from "@/lib/settings";

type Props = { chapterIds?: string[] };

export function KindleScreen({ chapterIds }: Props) {
  const nav = useNavigation();
  const [settings] = useSettings();
  const groups = useDownloadedChapters();
  const devices = useAsync(listDevices, []);
  const detectors = useAsync(detectorsStatus, []);
  const exportJob = useExport();

  const [selected, setSelected] = useState<Set<string>>(new Set(chapterIds ?? []));
  const [device, setDevice] = useState(settings.defaultDevice);
  const [preset, setPreset] = useState<Preset>(settings.defaultPreset);
  const [useDetectors, setUseDetectors] = useState(true);
  const [output, setOutput] = useState(`${settings.dataDir}/kindle`);

  useEffect(() => {
    if (detectors.state.kind === "ready" && !detectors.state.data.downloaded)
      setUseDetectors(false);
  }, [detectors.state]);

  const allChapters =
    groups.state.kind === "ready" ? groups.state.data.flatMap((g) => g.chapters) : [];
  const picked = allChapters.filter((c) => selected.has(c.id) && c.local_path);
  const picks =
    groups.state.kind === "ready"
      ? groups.state.data.map((g) => ({
          manga: g.manga,
          chapters: g.chapters.filter((c) => selected.has(c.id) && c.local_path),
        }))
      : [];
  const busy = exportJob.state.kind === "running";

  const chooseOutput = async () => {
    const dir = await pickFolder(
      "Choose where to write the EPUB files",
      `${settings.dataDir}/kindle`,
    );
    if (dir) setOutput(dir);
  };

  const run = () => {
    const title = exportTitle(picks);
    exportJob.run({
      chapter_dirs: picked.flatMap((c) => (c.local_path ? [c.local_path] : [])),
      device,
      preset,
      detectors: useDetectors,
      output: joinPath(output, exportFileName(title)),
      title,
    });
  };

  const detectorHint =
    detectors.state.kind !== "ready"
      ? "Checking detector support…"
      : detectors.state.data.available
        ? `Speech-bubble and face detectors (${detectors.state.data.provider}) keep cuts away from dialogue and faces. ${
            detectors.state.data.downloaded ? "Models are ready." : "Models download on first use."
          }`
        : "This build has no detector support; cuts follow gutters and panel edges only.";

  return (
    <section className="flex h-full flex-col">
      <ScreenHeader title="Send to Kindle" subtitle={`${picked.length} chapters picked`}>
        <Button
          variant="primary"
          size="sm"
          disabled={busy || picked.length === 0 || !output}
          onClick={run}
        >
          <Send /> Export{" "}
          {picked.length > 0
            ? `${picked.length} ${picked.length === 1 ? "chapter" : "chapters"}`
            : ""}
        </Button>
      </ScreenHeader>
      <div className="grid min-h-0 flex-1 grid-cols-[minmax(280px,2fr)_minmax(360px,3fr)]">
        <div className="overflow-auto border-r border-stroke p-4">
          {groups.state.kind === "loading" && (
            <p className="text-ink-3">Looking for downloaded chapters…</p>
          )}
          {groups.state.kind === "error" && <p className="text-accent">{groups.state.message}</p>}
          {groups.state.kind === "ready" && groups.state.data.length === 0 && (
            <EmptyState
              title="Nothing downloaded yet"
              action={<Button onClick={() => nav.go({ screen: "library" })}>Go to library</Button>}
            >
              Download chapters from a manga's page, then pick them here.
            </EmptyState>
          )}
          {groups.state.kind === "ready" && groups.state.data.length > 0 && (
            <DownloadedPicker
              groups={groups.state.data}
              selected={selected}
              onChange={setSelected}
            />
          )}
        </div>
        <div className="flex flex-col gap-5 overflow-auto p-4">
          <Field label="Device" htmlFor="device">
            {devices.state.kind === "ready" ? (
              <DeviceSelect
                id="device"
                devices={devices.state.data}
                value={device}
                onChange={setDevice}
              />
            ) : (
              <p className="text-ink-3">Loading devices…</p>
            )}
          </Field>
          <Field
            label="Cut preset"
            hint="Long strips are cut into pages at gutters; a preset decides how hard to avoid cutting through art."
          >
            <PresetPicker value={preset} onChange={setPreset} />
          </Field>
          <div className="flex items-start justify-between gap-4">
            <div>
              <label htmlFor="detectors" className="text-body font-medium">
                Use detectors
              </label>
              <p className="mt-0.5 text-caption text-ink-3">{detectorHint}</p>
            </div>
            <Switch
              id="detectors"
              checked={useDetectors}
              onCheckedChange={setUseDetectors}
              disabled={detectors.state.kind !== "ready" || !detectors.state.data.available}
            />
          </div>
          <Field label="Output folder" htmlFor="output">
            <div className="flex gap-2">
              <Input
                id="output"
                value={output}
                onChange={(e) => setOutput(e.target.value)}
                className="font-mono text-caption"
              />
              <Button onClick={chooseOutput} aria-label="Choose output folder">
                <FolderOpen /> Choose…
              </Button>
            </div>
          </Field>
          <ExportProgressPanel state={exportJob.state} onReset={exportJob.reset} />
        </div>
      </div>
    </section>
  );
}
