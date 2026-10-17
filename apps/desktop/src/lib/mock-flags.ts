/**
 * Query flags for the browser mock (`pnpm dev:web`), to reach states that are hard to trigger:
 * `?empty` starts with an empty library, `?offline` fails every search with a network error.
 * Searching for "offline" fails too.
 */
const params = new URLSearchParams(typeof window === "undefined" ? "" : window.location.search);

export const mockFlags = { emptyLibrary: params.has("empty"), offline: params.has("offline") };
