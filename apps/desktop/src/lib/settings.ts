/**
 * User preferences. Held in memory for synchronous reads from screens, hydrated once from the
 * backend at startup (`loadSettings`) and written through on every change.
 */
import { useCallback, useSyncExternalStore } from "react";
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
};

const DEFAULTS: Settings = {
  dataDir: "~/.local/share/mangas",
  defaultDevice: "kindle-paperwhite12",
  defaultPreset: DEFAULT_PRESET,
  preferredLanguages: [DEFAULT_LANGUAGE],
  providerOptions: {},
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
});

const notify = () => {
  for (const l of listeners) l();
};

export const readSettings = (): Settings => cached;

/** Hydrates from the backend; the platform data dir replaces the `~` placeholder default. */
export async function loadSettings(): Promise<void> {
  const [stored, platform] = await Promise.all([getSettings(), defaultDataDir()]);
  platformDataDir = platform || DEFAULTS.dataDir;
  cached = fromStored(stored, platformDataDir);
  notify();
}

export function writeSettings(patch: Partial<Settings>) {
  cached = { ...cached, ...patch };
  notify();
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
