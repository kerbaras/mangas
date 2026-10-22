import { tr } from "@/i18n/tr";
import type { Manga } from "./ipc";

/**
 * "2018 · Chugong · Art by Jang Sung-Rak". The year leads so a long credit list can't truncate
 * it; artists who are also the authors aren't repeated.
 */
export function creditsLine({ authors = [], artists = [], year }: Manga): string {
  const artOnly = artists.filter((a) => !authors.includes(a));
  return [
    year ? String(year) : "",
    authors.join(", "),
    artOnly.length > 0 ? tr("meta.artBy", { names: artOnly.join(", ") }) : "",
  ]
    .filter(Boolean)
    .join(" · ");
}

/** The first `max` tags and how many were left out. */
export function visibleTags({ tags = [] }: Manga, max: number) {
  return { shown: tags.slice(0, max), hidden: Math.max(0, tags.length - max) };
}
