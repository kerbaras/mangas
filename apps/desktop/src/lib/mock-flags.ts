import type { ExportResult } from "./ipc";

/**
 * Query flags for the browser mock (`pnpm dev:web`), to reach states that are hard to trigger:
 * `?empty` starts with an empty library, `?offline` fails every search with a network error,
 * `?skipped` makes every export leave out two unsupported files, `?down=weebcentral,…` makes those
 * providers unreachable. Searching for "offline" fails too.
 */
const params = new URLSearchParams(typeof window === "undefined" ? "" : window.location.search);

export const mockFlags = {
  emptyLibrary: params.has("empty"),
  offline: params.has("offline"),
  skipped: params.has("skipped"),
  down: (params.get("down") ?? "").split(",").filter(Boolean),
};

/** Throws the network error a real search gives when the source can't be reached. */
export function failIfOffline(source: string, query: string) {
  if (mockFlags.offline || query === "offline")
    throw new Error(`network: error sending request for url (${source}/search?q=${query})`);
}

/** Six pages per chapter; `?skipped` reports two unsupported files left out. */
export const mockExportResult = (output: string, chapters: number): ExportResult => ({
  output,
  pages: 6 * chapters,
  skipped: mockFlags.skipped ? 2 : 0,
});
