import { FolderOpen } from "lucide-react";
import { useState } from "react";
import { Field } from "@/components/field";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { parentFolder } from "@/lib/chapter-folder";
import { pickFolder } from "@/lib/dialogs";
import { isWindows } from "@/lib/platform";
import { useSettings } from "@/lib/settings";

/**
 * The folder picked for this export, else `<data dir>/kindle` (follows the settings as they load).
 * `picked` is null for the default: the backend resolves that folder itself.
 */
export function useOutputFolder() {
  const [settings] = useSettings();
  const fallback = settings.dataDir ? `${settings.dataDir}/kindle` : "";
  const [picked, setPicked] = useState<string | null>(null);
  const resolved = exportFolder(fallback, picked);
  return { output: picked ?? fallback, picked, fallback, resolved, setOutput: setPicked };
}

/**
 * Where the backend writes (`in_export_folder`, `Path::is_absolute` per OS): an absolute folder as
 * typed, a relative one (spaces kept) under the default export folder, a blank one the default.
 */
export function exportFolder(fallback: string, picked: string | null, windows = isWindows()) {
  if (!picked?.trim()) return fallback;
  if (windows ? /^([A-Za-z]:|[/\\])[/\\]/.test(picked) : picked.startsWith("/")) return picked;
  return fallback ? `${fallback}/${picked}` : picked;
}

type Props = { value: string; fallback: string; onChange: (dir: string) => void };

export function OutputFolder({ value, fallback, onChange }: Props) {
  const choose = async () => {
    const start = parentFolder(exportFolder(fallback, value));
    const dir = await pickFolder("Choose where to write the exported files", fallback, start);
    if (dir) onChange(dir);
  };
  return (
    <Field label="Output folder" htmlFor="output">
      <div className="flex gap-2">
        <Input
          id="output"
          value={value}
          onChange={(e) => onChange(e.target.value)}
          placeholder="kindle in the data folder"
          className="font-mono text-caption"
        />
        <Button onClick={() => void choose()} aria-label="Choose output folder">
          <FolderOpen /> Choose…
        </Button>
      </div>
    </Field>
  );
}
