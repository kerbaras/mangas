import { Terminal } from "lucide-react";
import { useState } from "react";
import { Field } from "@/components/field";
import { SettingsGroup } from "@/components/settings-group";
import { ThemePicker } from "@/components/theme-picker";
import { Button } from "@/components/ui/button";
import { errorMessage } from "@/lib/format";
import { installCli } from "@/lib/ipc";
import { useSettings } from "@/lib/settings";

/** Settings → General: theme and the `mangas` shell command. */
export function GeneralSettings() {
  const [settings, update] = useSettings();
  const [cliNote, setCliNote] = useState<string | null>(null);
  const linkCli = async () => {
    try {
      setCliNote(`Installed: ${await installCli()}. Run \`mangas --help\` in a terminal.`);
    } catch (e: unknown) {
      setCliNote(errorMessage(e));
    }
  };
  return (
    <SettingsGroup
      title="General"
      resets={["theme"]}
      resetHint="The theme follows your system again."
    >
      <Field label="Theme" hint="System follows your OS's light or dark mode.">
        <ThemePicker value={settings.theme} onChange={(theme) => update({ theme })} />
      </Field>
      <Field
        label="Shell command"
        hint="Links this app as `mangas` on your PATH so the same commands work from a terminal."
      >
        <div className="flex flex-col gap-2">
          <Button onClick={linkCli} className="self-start">
            <Terminal /> Install `mangas` command
          </Button>
          {cliNote && <p className="text-caption text-ink-3">{cliNote}</p>}
        </div>
      </Field>
    </SettingsGroup>
  );
}
