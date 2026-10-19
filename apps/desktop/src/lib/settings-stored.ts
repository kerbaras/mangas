/** The app's view of the settings and its mapping to the stored (backend) shape. */
import { isFormat } from "./export-formats";
import {
  DEFAULT_LANGUAGE,
  type ExportFormat,
  type Preset,
  type ProxyMode,
  type Settings as Stored,
  type Theme,
} from "./ipc";
import { EMPTY_EMAIL, type EmailSettings } from "./ipc.send";
import { DEFAULT_PRESET, isPreset } from "./presets";

export type Settings = {
  dataDir: string;
  defaultDevice: string;
  defaultPreset: Preset;
  /** Export format picked last; null follows what the device reads best. */
  defaultFormat: ExportFormat | null;
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
  kindleEmail: EmailSettings;
  theme: Theme;
  /** Chapters downloading at once (1–8); a provider gets half of them, at least 2. */
  downloadConcurrency: number;
  proxyMode: ProxyMode;
  /** Empty when none was typed; kept while another mode is picked. */
  proxyUrl: string;
};

export const DEFAULT_CONCURRENCY = 4;

export const DEFAULTS: Settings = {
  dataDir: "",
  defaultDevice: "kindle-paperwhite12",
  defaultPreset: DEFAULT_PRESET,
  defaultFormat: null,
  preferredLanguages: [DEFAULT_LANGUAGE],
  providerOptions: {},
  providersEnabled: {},
  providerLanguages: {},
  updateCheckHours: 6,
  updateNotifications: true,
  kindleEmail: EMPTY_EMAIL,
  theme: "system",
  downloadConcurrency: DEFAULT_CONCURRENCY,
  proxyMode: "system",
  proxyUrl: "",
};

const THEMES: readonly Theme[] = ["light", "dark", "system"];
const PROXY_MODES: readonly ProxyMode[] = ["system", "none", "custom"];
const clampConcurrency = (n: number) => Math.min(8, Math.max(1, Math.round(n)));

export const fromStored = (s: Stored, platformDataDir: string): Settings => ({
  dataDir: s.data_dir || platformDataDir,
  defaultDevice: s.default_device || DEFAULTS.defaultDevice,
  defaultPreset: isPreset(s.default_preset) ? s.default_preset : DEFAULTS.defaultPreset,
  defaultFormat: isFormat(s.default_format) ? s.default_format : null,
  preferredLanguages: s.preferred_languages?.length
    ? s.preferred_languages
    : DEFAULTS.preferredLanguages,
  providerOptions: s.provider_options ?? {},
  providersEnabled: s.providers_enabled ?? {},
  providerLanguages: s.provider_languages ?? {},
  updateCheckHours: s.update_check_hours ?? DEFAULTS.updateCheckHours,
  updateNotifications: s.update_notifications ?? DEFAULTS.updateNotifications,
  kindleEmail: { ...EMPTY_EMAIL, ...s.kindle_email },
  theme: s.theme && THEMES.includes(s.theme) ? s.theme : DEFAULTS.theme,
  downloadConcurrency: clampConcurrency(s.download_concurrency ?? DEFAULT_CONCURRENCY),
  proxyMode: s.proxy_mode && PROXY_MODES.includes(s.proxy_mode) ? s.proxy_mode : "system",
  proxyUrl: s.proxy_url ?? "",
});

/** Defaults are stored as `null`, so the stored settings only hold what was changed. */
const unlessDefault = <T>(value: T, fallback: T): T | null => (value === fallback ? null : value);

/** Provider sessions are owned by the backend (`set_settings` keeps the stored ones). */
export const toStored = (s: Settings, platformDataDir: string): Stored => ({
  provider_sessions: {},
  data_dir: s.dataDir === platformDataDir ? null : s.dataDir,
  default_device: unlessDefault(s.defaultDevice, DEFAULTS.defaultDevice),
  default_preset: unlessDefault(s.defaultPreset, DEFAULTS.defaultPreset),
  default_format: s.defaultFormat,
  preferred_languages: s.preferredLanguages,
  provider_options: s.providerOptions,
  providers_enabled: s.providersEnabled,
  provider_languages: s.providerLanguages,
  update_check_hours: s.updateCheckHours,
  update_notifications: s.updateNotifications,
  kindle_email: s.kindleEmail,
  theme: unlessDefault(s.theme, DEFAULTS.theme),
  download_concurrency: unlessDefault(s.downloadConcurrency, DEFAULT_CONCURRENCY),
  proxy_mode: unlessDefault(s.proxyMode, DEFAULTS.proxyMode),
  proxy_url: s.proxyUrl.trim() || null,
});

/** The stored field behind each app setting, so a write sends only the fields it changed. */
export const STORED_KEY: { [K in keyof Settings]: keyof Stored } = {
  dataDir: "data_dir",
  defaultDevice: "default_device",
  defaultPreset: "default_preset",
  defaultFormat: "default_format",
  preferredLanguages: "preferred_languages",
  providerOptions: "provider_options",
  providersEnabled: "providers_enabled",
  providerLanguages: "provider_languages",
  updateCheckHours: "update_check_hours",
  updateNotifications: "update_notifications",
  kindleEmail: "kindle_email",
  theme: "theme",
  downloadConcurrency: "download_concurrency",
  proxyMode: "proxy_mode",
  proxyUrl: "proxy_url",
};

/** The defaults of `keys`: a reset writes them, which stores each as unset. */
export const defaultsOf = (keys: readonly (keyof Settings)[]): Partial<Settings> =>
  Object.fromEntries(keys.map((k) => [k, DEFAULTS[k]]));
