/** Domain shapes the screens share (library, providers, search, download rows). */
import type { JobState } from "./ipc.downloads";

/** Mirrors `mangas_core::domain::Manga`. */
export type Manga = {
  id: string;
  source: string;
  title: string;
  description: string;
  cover_url: string | null;
  status: "ongoing" | "completed" | "hiatus" | "cancelled" | "unknown";
  /** Credits, first-publication year and genre tags; empty when the source doesn't publish them. */
  authors?: string[];
  artists?: string[];
  year?: number | null;
  tags?: string[];
};

/** Mirrors `mangas_core::domain::Chapter`. */
export type Chapter = {
  id: string;
  manga_id: string;
  title: string | null;
  language: string;
  volume: string | null;
  /** Chapter number as published ("12", "12.5"). */
  number: string;
  /** Directory with the downloaded pages, once downloaded. */
  local_path: string | null;
  /** Scanlation group(s) credited by the source ("A, B" when several). */
  group?: string | null;
  /** Whether the source lets this chapter be downloaded; absent means available. */
  availability?: Availability;
  /** When the chapter was last part of a successful export (UTC, RFC 3339). */
  exported_at?: string | null;
};

/** Mirrors `mangas_core::domain::Availability` (serde tag = "kind"). */
export type Availability =
  | { kind: "available" }
  | { kind: "locked" }
  | { kind: "external"; url: string | null };

/** Mirrors `mangas_core::smartcut::Preset` (kebab-case on the wire). */
export type Preset = "balanced" | "fewest-cuts" | "no-shrink";

/** Mirrors `mangas_providers::ProviderInfo`. */
export type ProviderInfo = {
  id: string;
  name: string;
  site: string;
  languages: string[];
  capabilities: ("search" | "chapters" | "download")[];
  needs_browser_session: boolean;
  icon_url: string | null;
};

/** Source name of manga imported from disk (`mangas_core::library::LOCAL_SOURCE`). */
export const LOCAL_SOURCE = "local";
export const isLocal = (m: { source: string }) => m.source === LOCAL_SOURCE;

/** Mirrors `mangas_core::library::ImportReport`. */
export type ImportReport = {
  manga: Manga;
  imported: { number: string; title: string | null; pages: number; path: string }[];
  skipped: { name: string; reason: string }[];
};

/** Mirrors `mangas_core::sources::ProviderStatus`. */
export type ProviderStatus =
  | { state: "ready" }
  | { state: "needs_session" }
  | { state: "unreachable"; reason: string };

/** One provider's share of a search (`mangas_core::sources::SearchGroup`): results when ready. */
export type SearchGroup = { provider: string; status: ProviderStatus; items: Manga[] };

/** Errors from a provider that needs a browser session start with this marker. */
export const CHALLENGE_PREFIX = "challenge:";
export const isChallenge = (message: string) => message.includes(CHALLENGE_PREFIX);

/** A chapter's download as its row shows it (see `toDownloadProgress`). */
export type DownloadProgress = {
  chapter_id: string;
  state: JobState;
  /** Why the last attempt failed, while a queued job waits to try again. */
  retrying: string | null;
  page: number;
  pages: number;
  done: boolean;
  error: string | null;
};
