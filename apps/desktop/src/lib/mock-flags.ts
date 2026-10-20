import type { ExportResult } from "./ipc";

/**
 * Query flags for the browser mock (`pnpm dev:web`), to reach states that are hard to trigger:
 * `?empty` starts with an empty library, `?offline` fails every search with a network error,
 * `?held` holds the download queue as a data-folder move would, `?quiet-queue` starts without
 * the simulated downloads, `?skipped` makes every export leave out two unsupported files, `?down=weebcentral,…` makes those
 * providers unreachable, `?fresh` starts as a fresh install (first-run setup), `?settings-fail` fails settings loads for the first 1.5 s (Retry then works).
 * App update flags are listed in `mock-app-update.ts`.
 * Detector flags are listed in `mock-detectors.ts`. Searching for "offline" fails too.
 */
const params = new URLSearchParams(typeof window === "undefined" ? "" : window.location.search);

export const mockFlags = {
  emptyLibrary: params.has("empty"),
  offline: params.has("offline"),
  skipped: params.has("skipped"),
  noDetectors: params.has("no-detectors"),
  detectorsMissing: params.has("detectors-missing"),
  detectorsFail: params.has("detectors-fail"),
  down: (params.get("down") ?? "").split(",").filter(Boolean),
  settingsFail: params.has("settings-fail"),
  fresh: params.has("fresh"),
  held: params.has("held"),
  update: params.has("update"),
  updateFail: params.has("update-fail"),
  installFail: params.has("install-fail"),
  quietQueue: params.has("quiet-queue") || params.has("empty"),
};

/** Throws the network error a real search gives when the source can't be reached. */
export function failIfOffline(source: string, query: string) {
  if (mockFlags.offline || query === "offline")
    throw new Error(`network: error sending request for url (${source}/search?q=${query})`);
}

/** Six pages per chapter; `?skipped` reports two unsupported files left out. */
export const mockExportResult = (output: string, chapters: number): ExportResult => ({
  output,
  parts: [],
  pages: 6 * chapters,
  skipped: mockFlags.skipped ? 2 : 0,
  detectors_error: null,
});
