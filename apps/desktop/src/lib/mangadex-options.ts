/** Mirrors `mangas_providers::mangadex::MangaDexOptions`, stored in `provider_options.mangadex`. */
export const MANGADEX = "mangadex";

export const CONTENT_RATINGS = ["safe", "suggestive", "erotica", "pornographic"] as const;
export type ContentRating = (typeof CONTENT_RATINGS)[number];

export type MangaDexOptions = {
  content_ratings: ContentRating[];
  data_saver: boolean;
  preferred_groups: string[];
};

export const MANGADEX_DEFAULTS: MangaDexOptions = {
  content_ratings: ["safe", "suggestive"],
  data_saver: false,
  preferred_groups: [],
};

const isRating = (v: unknown): v is ContentRating =>
  typeof v === "string" && (CONTENT_RATINGS as readonly string[]).includes(v);

const strings = (v: unknown): string[] | undefined =>
  Array.isArray(v) ? v.filter((x): x is string => typeof x === "string") : undefined;

/** Reads the stored entry; anything missing or malformed falls back to the default. */
export function parseMangaDexOptions(raw: unknown): MangaDexOptions {
  const o = typeof raw === "object" && raw !== null ? (raw as Record<string, unknown>) : {};
  const ratings = strings(o.content_ratings)?.filter(isRating);
  return {
    content_ratings: ratings?.length ? ratings : MANGADEX_DEFAULTS.content_ratings,
    data_saver: typeof o.data_saver === "boolean" ? o.data_saver : MANGADEX_DEFAULTS.data_saver,
    preferred_groups: strings(o.preferred_groups) ?? MANGADEX_DEFAULTS.preferred_groups,
  };
}

/** Toggles a rating, keeping MangaDex's order and at least one rating selected. */
export function toggleRating(current: ContentRating[], rating: ContentRating): ContentRating[] {
  const next = current.includes(rating)
    ? current.filter((r) => r !== rating)
    : [...current, rating];
  return next.length ? CONTENT_RATINGS.filter((r) => next.includes(r)) : current;
}

/** "A, B ,, c" -> ["A", "B", "c"]. */
export const parseGroups = (text: string): string[] =>
  text
    .split(",")
    .map((g) => g.trim())
    .filter(Boolean);

/** Toggles a language, keeping the order picked (first = most preferred) and at least one. */
export function toggleLanguage(current: string[], language: string): string[] {
  const next = current.includes(language)
    ? current.filter((l) => l !== language)
    : [...current, language];
  return next.length ? next : current;
}
