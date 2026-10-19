/**
 * User preferences. Held in memory for synchronous reads from screens, loaded from the backend
 * at startup and whenever the window comes back to the front (`loadSettings`: the CLI or a
 * restore may have changed them), and written through on every change. A write sends the copy
 * it was edited from, so the backend only saves the fields this window changed.
 */
import { useCallback, useSyncExternalStore } from "react";
import { errorMessage } from "./format";
import { defaultDataDir, getSettings, type Settings as Stored, setSettings } from "./ipc";
import { DEFAULTS, fromStored, type Settings, STORED_KEY, toStored } from "./settings-stored";

export type { Settings } from "./settings-stored";
export { DEFAULTS as SETTINGS_DEFAULTS, defaultsOf } from "./settings-stored";

const listeners = new Set<() => void>();
let cached: Settings = DEFAULTS;

/** Platform default; a `dataDir` equal to it is stored as `null` so the default can move. */
let platformDataDir = DEFAULTS.dataDir;

/** `base` with only the patched fields taken from `cached`: unset defaults stay unset. */
const edit = (base: Stored, patch: Partial<Settings>): Stored => {
  const next = toStored(cached, platformDataDir);
  const keys = Object.keys(patch) as (keyof Settings)[];
  return { ...base, ...Object.fromEntries(keys.map((k) => [STORED_KEY[k], next[STORED_KEY[k]]])) };
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
 * Bumped when a load succeeds after a failed first load let fetches run on placeholder
 * defaults: screens that fetched then (Details, Search) fetch again on it.
 */
let known = 0;
let releasedOnPlaceholders = false;
export const readSettingsKnown = (): number => known;

let firstSettled: () => void = () => {};
const firstLoad = new Promise<void>((resolve) => {
  firstSettled = resolve;
});

/**
 * Resolves once the first load answered (loaded or failed) and the writes made so far reached
 * the backend: a fetch awaiting it runs with the settings on screen, not placeholder defaults or
 * the ones a save in flight is about to replace.
 */
export async function settingsSettled(): Promise<void> {
  await firstLoad;
  let last: Promise<void>;
  do {
    last = writes;
    await last;
  } while (last !== writes);
}

/**
 * The SMTP password the email settings read from the keychain (`emailPassword`). Reloads never
 * read it (a locked keychain would prompt on every focus), so they keep this one in its place.
 */
let keychainPassword = "";
const withPassword = (s: Stored): Stored =>
  s.kindle_email && !s.kindle_email.password && keychainPassword
    ? { ...s, kindle_email: { ...s.kindle_email, password: keychainPassword } }
    : s;

/** Fills in the password the email settings read from the keychain. */
export function adoptKeychainPassword(password: string) {
  keychainPassword = password;
  if (!password || cached.kindleEmail.password) return;
  if (stored) stored = withPassword(stored);
  cached = { ...cached, kindleEmail: { ...cached.kindleEmail, password } };
  notify();
}

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
  // Asked apart from the settings: an unreadable row still leaves the real default folder known.
  const platform = defaultDataDir().catch(() => DEFAULTS.dataDir);
  try {
    const [loaded, dir] = await Promise.all([getSettings(), platform]);
    if (gen !== version && load.kind === "ready") return;
    platformDataDir = dir;
    stored = withPassword(loaded);
    cached = fromStored(stored, platformDataDir);
    load = { kind: "ready" };
    if (releasedOnPlaceholders) known++;
    releasedOnPlaceholders = false;
  } catch (e: unknown) {
    if (gen !== version || load.kind === "ready") return;
    load = { kind: "failed", error: errorMessage(e) };
    releasedOnPlaceholders = true;
    platformDataDir = await platform;
    cached = { ...cached, dataDir: cached.dataDir || platformDataDir };
  }
  notify();
  firstSettled();
}

/**
 * Writes through only once loaded, so placeholder defaults never overwrite what is stored.
 * The answer (this edit over what another process stored meanwhile) becomes the new copy.
 */
export function writeSettings(patch: Partial<Settings>): Promise<boolean> {
  cached = { ...cached, ...patch };
  notify();
  if (load.kind !== "ready") return Promise.resolve(false);
  const gen = ++version;
  const write = writes.then(async () => {
    const base = stored ?? toStored(cached, platformDataDir);
    const saved = await setSettings(edit(base, patch), base);
    keychainPassword = saved.kindle_email?.password ?? "";
    stored = saved;
    if (gen !== version) return;
    cached = fromStored(saved, platformDataDir);
    notify();
  });
  writes = write.catch((e: unknown) => console.warn("settings not saved", e));
  return write.then(
    () => true,
    () => false,
  );
}

/** Resolves once the writes queued so far are answered. */
export const settingsWritten = (): Promise<void> => writes;

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

/** Changes when settings loaded after fetches ran on placeholders: a dependency to refetch on. */
export const useSettingsKnown = (): number =>
  useSyncExternalStore(subscribe, readSettingsKnown, readSettingsKnown);
