import { useCallback, useSyncExternalStore } from "react";
import type { Preset } from "./ipc";
import { DEFAULT_PRESET, isPreset } from "./presets";

export type Settings = {
  dataDir: string;
  defaultDevice: string;
  defaultPreset: Preset;
};

const KEY = "mangas.settings";
const DEFAULTS: Settings = {
  dataDir: "~/.local/share/mangas",
  defaultDevice: "kindle-paperwhite12",
  defaultPreset: DEFAULT_PRESET,
};

const listeners = new Set<() => void>();
let cached: Settings | null = null;

function parse(raw: string | null): Settings {
  if (!raw) return DEFAULTS;
  try {
    const v = JSON.parse(raw) as Partial<Settings>;
    return {
      dataDir: typeof v.dataDir === "string" && v.dataDir ? v.dataDir : DEFAULTS.dataDir,
      defaultDevice: typeof v.defaultDevice === "string" ? v.defaultDevice : DEFAULTS.defaultDevice,
      defaultPreset: isPreset(v.defaultPreset) ? v.defaultPreset : DEFAULTS.defaultPreset,
    };
  } catch {
    return DEFAULTS;
  }
}

export function readSettings(): Settings {
  if (!cached) cached = parse(globalThis.localStorage?.getItem(KEY) ?? null);
  return cached;
}

export function writeSettings(patch: Partial<Settings>) {
  cached = { ...readSettings(), ...patch };
  globalThis.localStorage?.setItem(KEY, JSON.stringify(cached));
  for (const l of listeners) l();
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
