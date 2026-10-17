/**
 * Query flags for the browser mock (`pnpm dev:web`), to reach states that are hard to trigger:
 * `?empty` starts with an empty library, `?offline` fails every search with a network error.
 * Searching for "offline" fails too.
 */
const params = new URLSearchParams(typeof window === "undefined" ? "" : window.location.search);

export const mockFlags = { emptyLibrary: params.has("empty"), offline: params.has("offline") };

/** Throws the network error a real search gives when the source can't be reached. */
export function failIfOffline(source: string, query: string) {
  if (mockFlags.offline || query === "offline")
    throw new Error(`network: error sending request for url (${source}/search?q=${query})`);
}
