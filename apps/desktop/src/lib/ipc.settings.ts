/** Settings part of the IPC contract. */
import type { Preset } from "./ipc";

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
  provider_sessions: Record<string, ProviderSession>;
  /** Chapter languages, most preferred first (empty = English); options by provider id. */
  preferred_languages?: string[];
  provider_options?: Record<string, unknown>;
};
