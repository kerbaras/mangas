import { tr } from "@/i18n/tr";
import type { Manga } from "./ipc";

/**
 * "2018 · Chugong · Art by Jang Sung-Rak". The year leads so a long credit list can't truncate
 * it; artists who are also the authors aren't repeated.
 */
/** Who a book of this title credits, as `Manga::creators` in core: authors, then artists not
 * already named, trimmed and each once. Empty when the source names nobody. */
export function bookCreators({ authors = [], artists = [] }: Pick<Manga, "authors" | "artists">) {
  const out: string[] = [];
  for (const name of [...authors, ...artists].map((n) => n.trim())) {
    if (name && !out.includes(name)) out.push(name);
  }
  return out;
}

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
