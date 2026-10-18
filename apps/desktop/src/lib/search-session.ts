/**
 * The last search, kept outside the screen so Back from a manga's details lands on the same
 * query and results without searching again (the provider filter is kept in local storage).
 */
import type { GroupState } from "./search-groups";

type Groups = Record<string, GroupState>;
type Results = { query: string; providers: string; groups: Groups };

let input = "";
let results: Results | null = null;

export const searchInput = () => input;

export function rememberInput(query: string) {
  input = query;
}

/** Groups for a trimmed query over a provider list (`ids.join(",")`); empty ones aren't kept. */
export function rememberResults(query: string, providers: string, groups: Groups) {
  if (query && providers && Object.keys(groups).length > 0) results = { query, providers, groups };
}

export const cachedResults = (query: string, providers: string): Groups | null =>
  results && results.query === query && results.providers === providers ? results.groups : null;

export function forgetSearch() {
  input = "";
  results = null;
}
