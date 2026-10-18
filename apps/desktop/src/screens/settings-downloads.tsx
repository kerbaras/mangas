import { Field } from "@/components/field";
import { LanguagesPicker } from "@/components/languages-picker";
import { MangaDexOptionsForm } from "@/components/mangadex-options";
import { ProvidersList } from "@/components/providers-list";
import { SettingsGroup } from "@/components/settings-group";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { UpdatesSettings } from "@/components/updates-settings";
import { useSettings } from "@/lib/settings";

const CONCURRENCY = [1, 2, 3, 4, 5, 6, 7, 8];
/** Mirrors `Limits::for_concurrency`: half per site, at least 2. */
const perSite = (n: number) => Math.min(n, Math.max(2, Math.ceil(n / 2)));

/** Settings → Downloads: how many at once, which sources, languages and content. */
export function DownloadsSettings() {
  const [settings, update] = useSettings();
  const n = settings.downloadConcurrency;
  return (
    <SettingsGroup
      title="Downloads"
      resets={[
        "downloadConcurrency",
        "preferredLanguages",
        "providerLanguages",
        "providerOptions",
        "providersEnabled",
        "updateCheckHours",
        "updateNotifications",
      ]}
      resetHint="Concurrency, languages, provider options and update checks go back to their defaults, and every provider is switched on. Provider sessions are kept."
    >
      <Field
        label="Chapters at once"
        htmlFor="download-concurrency"
        hint={`Up to ${perSite(n)} from the same site. Applies to the running queue straight away.`}
      >
        <Select value={String(n)} onValueChange={(v) => update({ downloadConcurrency: Number(v) })}>
          <SelectTrigger id="download-concurrency" className="w-24">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {CONCURRENCY.map((c) => (
              <SelectItem key={c} value={String(c)}>
                {c}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </Field>
      <Field
        label="Content languages"
        hint="Chapters are listed in these languages, numbered by preference; search only shows titles translated into one of them. A provider can use its own from its row below."
      >
        <LanguagesPicker />
      </Field>
      <Field label="MangaDex" hint="Content rating and other options of the MangaDex API.">
        <MangaDexOptionsForm />
      </Field>
      <Field
        label="Sources"
        hint="Search runs on the providers switched on. Sites behind a verification wall open in their own window once; the session is reused by the app and the `mangas` command until it expires."
      >
        <ProvidersList />
      </Field>
      <UpdatesSettings />
    </SettingsGroup>
  );
}
