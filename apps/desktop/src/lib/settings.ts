/**
 * User preferences. Held in memory for synchronous reads from screens, loaded from the backend
 * at startup and whenever the window comes back to the front (`loadSettings`: the CLI or a
 * restore may have changed them), and written through on every change. A write sends the copy
 * it was edited from, so the backend only saves the fields this window changed.
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
  /** Hours between background update checks; 0 turns them off. */
  updateCheckHours: number;
  updateNotifications: boolean;
};

const DEFAULTS: Settings = {
  dataDir: "~/.local/share/mangas",
  defaultDevice: "kindle-paperwhite12",
  defaultPreset: DEFAULT_PRESET,
  preferredLanguages: [DEFAULT_LANGUAGE],
  providerOptions: {},
  providersEnabled: {},
  providerLanguages: {},
  updateCheckHours: 6,
  updateNotifications: true,
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
  updateCheckHours: s.update_check_hours ?? DEFAULTS.updateCheckHours,
  updateNotifications: s.update_notifications ?? DEFAULTS.updateNotifications,
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
  update_check_hours: s.updateCheckHours,
  update_notifications: s.updateNotifications,
});

/** The stored field behind each app setting, so a write sends only the fields it changed. */
const STORED_KEY: { [K in keyof Settings]: keyof Stored } = {
  dataDir: "data_dir",
  defaultDevice: "default_device",
  defaultPreset: "default_preset",
  preferredLanguages: "preferred_languages",
  providerOptions: "provider_options",
  providersEnabled: "providers_enabled",
  providerLanguages: "provider_languages",
  updateCheckHours: "update_check_hours",
  updateNotifications: "update_notifications",
};

/** `base` with only the patched fields taken from `cached`: unset defaults stay unset. */
const edit = (base: Stored, patch: Partial<Settings>): Stored => {
  const next = toStored(cached);
  const keys = Object.keys(patch) as (keyof Settings)[];
  return keys.reduce((s, k) => ({ ...s, [STORED_KEY[k]]: next[STORED_KEY[k]] }), base);
};

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
/** Bumped by every load and write: only the latest one's answer replaces `cached`. */
let version = 0;
/** What the backend last said is stored: the base of the next write. */
let stored: Stored | undefined;
/** Writes go one at a time, each based on what the previous one stored. */
let writes: Promise<void> = Promise.resolve();
export const readSettingsLoad = (): SettingsLoad => load;

/**
 * Hydrates from the backend; the platform data dir replaces the `~` placeholder default. Never
 * throws: a failure is kept in `readSettingsLoad()` for the Settings screen to show and retry,
 * unless settings already loaded: an older answer never replaces a newer one, and a failed
 * reload doesn't hide loaded settings.
 */
export async function loadSettings(): Promise<void> {
  const gen = ++version;
  if (load.kind !== "ready") {
    load = { kind: "loading" };
    notify();
  }
  try {
    const [loaded, platform] = await Promise.all([getSettings(), defaultDataDir()]);
    if (gen !== version && load.kind === "ready") return;
    platformDataDir = platform || DEFAULTS.dataDir;
    stored = loaded;
    cached = fromStored(loaded, platformDataDir);
    load = { kind: "ready" };
  } catch (e: unknown) {
    if (gen !== version || load.kind === "ready") return;
    load = { kind: "failed", error: errorMessage(e) };
  }
  notify();
}

/**
 * Writes through only once loaded, so placeholder defaults never overwrite what is stored.
 * The answer (this edit over what another process stored meanwhile) becomes the new copy.
 */
export function writeSettings(patch: Partial<Settings>) {
  cached = { ...cached, ...patch };
  notify();
  if (load.kind !== "ready") return;
  const gen = ++version;
  writes = writes
    .then(async () => {
      const base = stored ?? toStored(cached);
      const saved = await setSettings(edit(base, patch), base);
      stored = saved;
      if (gen !== version) return;
      cached = fromStored(saved, platformDataDir);
      notify();
    })
    .catch((e: unknown) => console.warn("settings not saved", e));
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
