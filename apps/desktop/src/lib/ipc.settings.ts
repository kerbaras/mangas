/** Settings part of the IPC contract. */
import type { ExportFormat, Preset } from "./ipc";
import type { EmailSettings } from "./ipc.send";

/** Browser session captured for a provider behind a verification wall. */
export type ProviderSession = {
  cookies: string;
  user_agent: string;
  saved_at: string | null;
};

export type Theme = "light" | "dark" | "system";
/** `system` follows the OS proxy settings, `none` connects directly, `custom` uses `proxy_url`. */
export type ProxyMode = "system" | "none" | "custom";

/** Mirrors `mangas_core::library::Settings`; `null` means "use the default". */
export type Settings = {
  data_dir: string | null;
  default_device: string | null;
  default_preset: Preset | null;
  /** Export format picked last; null follows the device profile. */
  default_format?: ExportFormat | null;
  provider_sessions: Record<string, ProviderSession>;
  /** Chapter languages, most preferred first (empty = English); options by provider id. */
  preferred_languages?: string[];
  provider_options?: Record<string, unknown>;
  /** Providers switched off are `false`; a provider missing here is enabled. */
  providers_enabled?: Record<string, boolean>;
  /** Chapter languages per provider id; empty or missing = `preferred_languages`. */
  provider_languages?: Record<string, string[]>;
  /** Hours between background update checks; null = 6, 0 = off. */
  update_check_hours?: number | null;
  /** Desktop notification when a background check finds new chapters; null = on. */
  update_notifications?: boolean | null;
  /** Release channel the app updates from; null = stable. */
  app_update_channel?: "stable" | "rc" | null;
  /** Whether the app looks for a new version at launch; null = on. */
  app_update_checks?: boolean | null;
  /** Send to Kindle by email; the password is stored unencrypted. */
  kindle_email?: EmailSettings;
  /** null = system. */
  theme?: Theme | null;
  /** Chapters downloading at once, 1–8; null = 4. */
  download_concurrency?: number | null;
  /** null = system. */
  proxy_mode?: ProxyMode | null;
  /** Kept while another mode is picked, so switching back restores it. */
  proxy_url?: string | null;
  setup_complete?: boolean | null;
};
