/** Settings part of the IPC contract. */
import type { ExportFormat, Preset } from "./ipc";

/** Browser session captured for a provider behind a verification wall. */
export type ProviderSession = {
  cookies: string;
  user_agent: string;
  saved_at: string | null;
};

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
};
