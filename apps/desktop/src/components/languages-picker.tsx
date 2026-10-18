import { useAsync } from "@/hooks/use-async";
import { cn } from "@/lib/cn";
import { languageLabel } from "@/lib/format";
import { listProviders } from "@/lib/ipc";
import { toggleLanguage } from "@/lib/mangadex-options";
import { useSettings } from "@/lib/settings";

/** Every language some provider offers, as toggles; the order picked is the preference. */
export function LanguagesPicker() {
  const [settings, update] = useSettings();
  const providers = useAsync(listProviders, []);
  if (providers.state.kind !== "ready") return <p className="text-ink-3">Loading languages…</p>;
  const offered = [...new Set(providers.state.data.flatMap((p) => p.languages))];
  const picked = settings.preferredLanguages;
  return (
    <fieldset className="flex flex-wrap gap-1.5" aria-label="Chapter languages">
      {offered.map((code) => {
        const rank = picked.indexOf(code);
        const on = rank >= 0;
        return (
          <button
            key={code}
            type="button"
            aria-pressed={on}
            onClick={() => update({ preferredLanguages: toggleLanguage(picked, code) })}
            className={cn(
              "inline-flex h-7 items-center gap-1.5 rounded-md border px-2.5 text-caption transition-colors",
              on
                ? "border-accent bg-accent-soft text-accent"
                : "border-stroke text-ink-2 hover:bg-surface-2",
            )}
          >
            {on && picked.length > 1 && (
              <span className="tabular-nums font-semibold">{rank + 1}</span>
            )}
            {languageLabel(code)}
          </button>
        );
      })}
    </fieldset>
  );
}
