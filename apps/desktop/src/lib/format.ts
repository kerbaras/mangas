import type { Manga } from "./ipc";

export const statusLabel: Record<Manga["status"], string> = {
  ongoing: "Ongoing",
  completed: "Completed",
  hiatus: "On hiatus",
  cancelled: "Cancelled",
  unknown: "Unknown",
};

/** MangaDex suffixes that are not ISO regions: `es-la` is Latin America, not Laos. */
const REGION_SUFFIXES: Record<string, string> = { la: "Latin America", ro: "Romanized" };

const displayName = (code: string) => {
  try {
    return new Intl.DisplayNames(["en"], { type: "language" }).of(code) ?? code;
  } catch {
    return code;
  }
};

export const languageLabel = (code: string) => {
  const [base, suffix] = code.toLowerCase().split("-", 2);
  const region = suffix ? REGION_SUFFIXES[suffix] : undefined;
  return region ? `${displayName(base)} (${region})` : displayName(code);
};

/** "English", "English or Spanish", "English, French, or Spanish". */
export const languageList = (codes: string[]) =>
  new Intl.ListFormat("en", { type: "disjunction" }).format(codes.map(languageLabel));

export const mangaKey = (m: { source: string; id: string }) => `${m.source}:${m.id}`;

export const errorMessage = (e: unknown) => (e instanceof Error ? e.message : String(e));
