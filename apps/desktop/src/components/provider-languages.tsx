import { ChevronDown, Languages } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuCheckboxItem,
  DropdownMenuContent,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { languageLabel } from "@/lib/format";
import type { ProviderInfo } from "@/lib/ipc";
import { offersLanguagePick, toggled, withLanguages } from "@/lib/provider-prefs";
import { useSettings } from "@/lib/settings";

const keepOpen = (e: Event) => e.preventDefault();

/** Chapter languages for one provider: the app-wide list, or its own pick among what it offers. */
export function ProviderLanguages({ provider: p }: { provider: ProviderInfo }) {
  const [settings, update] = useSettings();
  const own = (settings.providerLanguages[p.id] ?? []).filter((l) => p.languages.includes(l));
  const set = (list: string[]) =>
    update({ providerLanguages: withLanguages(settings, p.id, list) });
  if (!offersLanguagePick(p.languages, settings.preferredLanguages)) return null;
  const label = own.length ? own.map((l) => l.toUpperCase()).join(", ") : "App languages";
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button size="sm" variant="ghost" aria-label={`Chapter languages for ${p.name}`}>
          <Languages /> {label} <ChevronDown className="text-ink-3" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        <DropdownMenuLabel>Chapter languages on {p.name}</DropdownMenuLabel>
        <DropdownMenuCheckboxItem
          checked={own.length === 0}
          onSelect={keepOpen}
          onCheckedChange={() => set([])}
        >
          Same as the app
        </DropdownMenuCheckboxItem>
        <DropdownMenuSeparator />
        {p.languages.map((code) => (
          <DropdownMenuCheckboxItem
            key={code}
            checked={own.includes(code)}
            onSelect={keepOpen}
            onCheckedChange={() => set(toggled(own, code))}
          >
            <span className="flex-1">{languageLabel(code)}</span>
            {own.length > 1 && own.includes(code) && (
              <span className="text-caption text-ink-3 tabular-nums">{own.indexOf(code) + 1}</span>
            )}
          </DropdownMenuCheckboxItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
