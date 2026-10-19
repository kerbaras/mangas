/**
 * Per-screen view state kept for the session (not across launches), so Back from a manga's
 * details lands where the list was left: the scroll offset of each list and the Library's title
 * filter. Category, filters, sort and layout are kept in local storage by their own hooks.
 */
const offsets = new Map<string, number>();
let libraryQuery = "";

export const savedScroll = (key: string): number => offsets.get(key) ?? 0;

export function rememberScroll(key: string, top: number) {
  offsets.set(key, top);
}

export const savedLibraryQuery = () => libraryQuery;

export function rememberLibraryQuery(query: string) {
  libraryQuery = query;
}

export function forgetScreens() {
  offsets.clear();
  libraryQuery = "";
}
