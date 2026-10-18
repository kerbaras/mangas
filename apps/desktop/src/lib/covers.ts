import { coverSrc, type Manga } from "./ipc";

/**
 * One cover request per manga and URL for the whole session, shared by every screen showing it.
 * A failed request, or a file that no longer loads (evicted from the disk cache), is forgotten,
 * so the next mount asks the backend again.
 */
const pending = new Map<string, Promise<string | null>>();

type CoverRef = Pick<Manga, "source" | "id" | "cover_url">;

const keyOf = (m: CoverRef) => `${m.source}:${m.id}:${m.cover_url}`;

export function loadCover(manga: CoverRef): Promise<string | null> {
  if (!manga.cover_url) return Promise.resolve(null);
  const key = keyOf(manga);
  const known = pending.get(key);
  if (known) return known;
  const request = coverSrc(manga).catch(() => {
    pending.delete(key);
    return null;
  });
  pending.set(key, request);
  return request;
}

export const forgetCover = (manga: CoverRef) => pending.delete(keyOf(manga));

/** Tests start from an empty session. */
export const resetCovers = () => pending.clear();
