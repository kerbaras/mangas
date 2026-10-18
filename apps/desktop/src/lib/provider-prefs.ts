/** Per-provider preferences read from the settings; mirrors `Settings::provider_enabled` in core. */
import type { ProviderInfo } from "./ipc";
import type { Settings } from "./settings";

type Prefs = Pick<Settings, "providersEnabled" | "providerLanguages" | "preferredLanguages">;

/** On unless switched off, so providers added in a later release start enabled. */
export const isEnabled = (s: Pick<Prefs, "providersEnabled">, id: string) =>
  s.providersEnabled[id] !== false;

/** The provider's own chapter languages when set, else the app-wide ones. */
export const languagesFor = (s: Prefs, id: string) => {
  const own = s.providerLanguages[id];
  return own?.length ? own : s.preferredLanguages;
};

/** Enabled providers that can search, in registry order. */
export const searchable = (providers: ProviderInfo[], s: Pick<Prefs, "providersEnabled">) =>
  providers.filter((p) => p.capabilities.includes("search") && isEnabled(s, p.id));

export const withEnabled = (s: Pick<Prefs, "providersEnabled">, id: string, on: boolean) => ({
  ...s.providersEnabled,
  [id]: on,
});

/** Sets `id`'s own languages; an empty list drops the entry so it follows the app-wide list. */
export const withLanguages = (s: Pick<Prefs, "providerLanguages">, id: string, list: string[]) => {
  const { [id]: _, ...rest } = s.providerLanguages;
  return list.length ? { ...rest, [id]: list } : rest;
};

/** Adds or removes `code`, keeping the pick order; may end empty (= follow the app list). */
export const toggled = (list: string[], code: string) =>
  list.includes(code) ? list.filter((l) => l !== code) : [...list, code];
