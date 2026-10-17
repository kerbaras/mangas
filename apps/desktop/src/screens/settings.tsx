import { FolderOpen, Terminal } from "lucide-react";
import { useState } from "react";
import { DeviceSelect } from "@/components/device-select";
import { DevicesPending } from "@/components/devices-pending";
import { DiagnosticsPanel } from "@/components/diagnostics-panel";
import { Field } from "@/components/field";
import { LanguagesPicker } from "@/components/languages-picker";
import { MangaDexOptionsForm } from "@/components/mangadex-options";
import { PresetPicker } from "@/components/preset-picker";
import { ProvidersList } from "@/components/providers-list";
import { ScreenHeader } from "@/components/screen-header";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { useAsync } from "@/hooks/use-async";
import { pickFolder } from "@/lib/dialogs";
import { errorMessage } from "@/lib/format";
import { installCli, listDevices } from "@/lib/ipc";
import { useSettings } from "@/lib/settings";

export function SettingsScreen() {
  const [settings, update] = useSettings();
  const devices = useAsync(listDevices, []);

  const [cliNote, setCliNote] = useState<string | null>(null);
  const linkCli = async () => {
    try {
      setCliNote(`Installed: ${await installCli()}. Run \`mangas --help\` in a terminal.`);
    } catch (e: unknown) {
      setCliNote(errorMessage(e));
    }
  };

  const chooseDataDir = async () => {
    const dir = await pickFolder("Choose the data folder", settings.dataDir);
    if (dir) update({ dataDir: dir });
  };

  return (
    <section className="flex h-full flex-col">
      <ScreenHeader title="Settings" />
      <div className="flex-1 overflow-auto p-4">
        <div className="mx-auto flex max-w-2xl flex-col gap-8">
          <Group title="Storage">
            <Field
              label="Data folder"
              htmlFor="data-dir"
              hint="Downloaded chapters and detector models live here; the library database stays in the app data folder."
            >
              <div className="flex gap-2">
                <Input
                  id="data-dir"
                  value={settings.dataDir}
                  onChange={(e) => update({ dataDir: e.target.value })}
                  className="font-mono text-caption"
                />
                <Button onClick={chooseDataDir} aria-label="Choose data folder">
                  <FolderOpen /> Choose…
                </Button>
              </div>
            </Field>
          </Group>
          <Group title="Providers">
            <Field
              label="Sources"
              hint="Sites behind a verification wall open in their own window once; the session is reused by the app and the `mangas` command until it expires."
            >
              <ProvidersList />
            </Field>
            <Field
              label="Chapter languages"
              hint="Chapters are listed in these languages, numbered by preference; search only shows titles translated into one of them."
            >
              <LanguagesPicker />
            </Field>
            <Field label="MangaDex">
              <MangaDexOptionsForm />
            </Field>
          </Group>
          <Group title="Kindle">
            <Field
              label="Default device"
              htmlFor="default-device"
              hint="Preselected on Send to Kindle."
            >
              {devices.state.kind === "ready" ? (
                <DeviceSelect
                  id="default-device"
                  devices={devices.state.data}
                  value={settings.defaultDevice}
                  onChange={(id) => update({ defaultDevice: id })}
                />
              ) : (
                <DevicesPending state={devices.state} />
              )}
            </Field>
            <Field label="Default cut preset">
              <PresetPicker
                compact
                value={settings.defaultPreset}
                onChange={(p) => update({ defaultPreset: p })}
              />
            </Field>
          </Group>
          <Group title="Command line">
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
          </Group>
          <Group title="Diagnostics">
            <DiagnosticsPanel />
          </Group>
        </div>
      </div>
    </section>
  );
}

function Group({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <section className="flex flex-col gap-4">
      <h2 className="border-b border-stroke pb-2 text-caption font-semibold text-ink-3 uppercase tracking-wide">
        {title}
      </h2>
      {children}
    </section>
  );
}
