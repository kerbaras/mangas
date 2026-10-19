import { FolderOpen } from "lucide-react";
import { useState } from "react";
import { Field } from "@/components/field";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { pickFolder } from "@/lib/dialogs";
import { useSettings } from "@/lib/settings";

/**
 * The folder picked for this export, else `<data dir>/kindle` (follows the settings as they load).
 * `picked` is null for the default: the backend resolves that folder itself.
 */
export function useOutputFolder() {
  const [settings] = useSettings();
  const fallback = settings.dataDir ? `${settings.dataDir}/kindle` : "";
  const [picked, setPicked] = useState<string | null>(null);
  return { output: picked ?? fallback, picked, fallback, setOutput: setPicked };
}

type Props = { value: string; fallback: string; onChange: (dir: string) => void };

export function OutputFolder({ value, fallback, onChange }: Props) {
  const choose = async () => {
    const dir = await pickFolder("Choose where to write the exported files", fallback);
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
