import type { DownloadJob } from "@/lib/ipc.downloads";

/** A queue job with everything defaulted; `patch` sets what the test is about. */
export const makeJob = (patch: Partial<DownloadJob> = {}): DownloadJob => ({
  id: 1,
  source: "mangadex",
  manga_id: "m",
  chapter_id: `c${patch.id ?? 1}`,
  state: "queued",
  attempts: 0,
  error: null,
  pages_done: 0,
  pages_total: 0,
  not_before: 0,
  owner: null,
  heartbeat_at: 0,
  stale_lease: false,
  stalled: false,
  force: false,
  created_at: 0,
  updated_at: 0,
  manga_title: "Kaiju",
  number: "7",
  title: null,
  language: "en",
  local_path: null,
  ...patch,
});
