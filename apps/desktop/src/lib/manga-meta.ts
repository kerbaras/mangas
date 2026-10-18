import type { Manga } from "./ipc";

/** "Chugong · Art by Jang Sung-Rak · 2018"; artists who are also the authors aren't repeated. */
export function creditsLine({ authors = [], artists = [], year }: Manga): string {
  const artOnly = artists.filter((a) => !authors.includes(a));
  return [
    authors.join(", "),
    artOnly.length > 0 ? `Art by ${artOnly.join(", ")}` : "",
    year ? String(year) : "",
  ]
    .filter(Boolean)
    .join(" · ");
}

/** The first `max` tags and how many were left out. */
export function visibleTags({ tags = [] }: Manga, max: number) {
  return { shown: tags.slice(0, max), hidden: Math.max(0, tags.length - max) };
}
