import { coverSrc, type Manga } from "./ipc";

/**
 * One cover request per manga and URL for the whole session, shared by every screen showing it.
 * A failed request is forgotten, so the next mount tries again.
 */
const pending = new Map<string, Promise<string | null>>();

type CoverRef = Pick<Manga, "source" | "id" | "cover_url">;

export function loadCover(manga: CoverRef): Promise<string | null> {
  if (!manga.cover_url) return Promise.resolve(null);
  const key = `${manga.source}:${manga.id}:${manga.cover_url}`;
  const known = pending.get(key);
  if (known) return known;
  const request = coverSrc(manga).catch(() => {
    pending.delete(key);
    return null;
  });
  pending.set(key, request);
  return request;
}

/** Tests start from an empty session. */
export const resetCovers = () => pending.clear();
