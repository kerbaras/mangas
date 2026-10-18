/**
 * User preferences. Held in memory for synchronous reads from screens, hydrated once from the
 * backend at startup (`loadSettings`) and written through on every change.
 */
import { useCallback, useSyncExternalStore } from "react";
import { errorMessage } from "./format";
import {
  DEFAULT_LANGUAGE,
  defaultDataDir,
  getSettings,
  type Preset,
  type Settings as Stored,
  setSettings,
} from "./ipc";
import { DEFAULT_PRESET, isPreset } from "./presets";

export type Settings = {
  dataDir: string;
  defaultDevice: string;
  defaultPreset: Preset;
  /** Chapter languages in order of preference, never empty. */
  preferredLanguages: string[];
  providerOptions: Record<string, unknown>;
  /** Only providers switched off are listed (`false`); see `lib/provider-prefs.ts`. */
  providersEnabled: Record<string, boolean>;
  /** Chapter languages per provider, overriding `preferredLanguages` when not empty. */
  providerLanguages: Record<string, string[]>;
};

const DEFAULTS: Settings = {
  dataDir: "~/.local/share/mangas",
  defaultDevice: "kindle-paperwhite12",
  defaultPreset: DEFAULT_PRESET,
  preferredLanguages: [DEFAULT_LANGUAGE],
  providerOptions: {},
  providersEnabled: {},
  providerLanguages: {},
};

const listeners = new Set<() => void>();
let cached: Settings = DEFAULTS;

const fromStored = (s: Stored, platformDataDir: string): Settings => ({
  dataDir: s.data_dir || platformDataDir,
  defaultDevice: s.default_device || DEFAULTS.defaultDevice,
  defaultPreset: isPreset(s.default_preset) ? s.default_preset : DEFAULTS.defaultPreset,
  preferredLanguages: s.preferred_languages?.length
    ? s.preferred_languages
    : DEFAULTS.preferredLanguages,
  providerOptions: s.provider_options ?? {},
  providersEnabled: s.providers_enabled ?? {},
  providerLanguages: s.provider_languages ?? {},
});

/** Platform default; a `dataDir` equal to it is stored as `null` so the default can move. */
let platformDataDir = DEFAULTS.dataDir;

/** Provider sessions are owned by the backend (`set_settings` keeps the stored ones). */
const toStored = (s: Settings): Stored => ({
  provider_sessions: {},
  data_dir: s.dataDir === platformDataDir ? null : s.dataDir,
  default_device: s.defaultDevice,
  default_preset: s.defaultPreset,
  preferred_languages: s.preferredLanguages,
  provider_options: s.providerOptions,
  providers_enabled: s.providersEnabled,
  provider_languages: s.providerLanguages,
});

const notify = () => {
  for (const l of listeners) l();
};

export const readSettings = (): Settings => cached;

/** Whether the stored settings are in: until they are, `cached` holds placeholder defaults. */
export type SettingsLoad =
  | { kind: "loading" }
  | { kind: "ready" }
  | { kind: "failed"; error: string };
let load: SettingsLoad = { kind: "loading" };
export const readSettingsLoad = (): SettingsLoad => load;

/**
 * Hydrates from the backend; the platform data dir replaces the `~` placeholder default. Never
 * throws: a failure is kept in `readSettingsLoad()` for the Settings screen to show and retry,
 * unless settings already loaded (an overlapping or later reload can't hide them again).
 */
export async function loadSettings(): Promise<void> {
  if (load.kind !== "ready") {
    load = { kind: "loading" };
    notify();
  }
  try {
    const [stored, platform] = await Promise.all([getSettings(), defaultDataDir()]);
    platformDataDir = platform || DEFAULTS.dataDir;
    cached = fromStored(stored, platformDataDir);
    load = { kind: "ready" };
  } catch (e: unknown) {
    if (load.kind !== "ready") load = { kind: "failed", error: errorMessage(e) };
  }
  notify();
}

/** Writes through only once loaded, so placeholder defaults never overwrite what is stored. */
export function writeSettings(patch: Partial<Settings>) {
  cached = { ...cached, ...patch };
  notify();
  if (load.kind !== "ready") return;
  void setSettings(toStored(cached)).catch((e: unknown) => console.warn("settings not saved", e));
}

const subscribe = (l: () => void) => {
  listeners.add(l);
  return () => listeners.delete(l);
};

export function useSettings(): [Settings, (patch: Partial<Settings>) => void] {
  const settings = useSyncExternalStore(subscribe, readSettings, readSettings);
  const update = useCallback((patch: Partial<Settings>) => writeSettings(patch), []);
  return [settings, update];
}

export const useSettingsLoad = (): SettingsLoad =>
  useSyncExternalStore(subscribe, readSettingsLoad, readSettingsLoad);
