import type { Chapter } from "./ipc";

const SEASON_TAG = /^S\d+ (?:Ep\. \d+(?:\.\d+)?|\p{L}+)(?=$| )/u;

/**
 * The `S2 Ep. 5` or `S2 Prologue` prefix the core gives season-numbered titles (Webtoons), if
 * any. Mirrors `domain::season_tag`; `season-tag.test.ts` runs the core's fixture against it.
 */
export const seasonTag = (c: Chapter): string | null => c.title?.match(SEASON_TAG)?.[0] ?? null;

/** How a file name refers to one chapter: its season tag, else `ch 12`. */
export const chapterTag = (c: Chapter): string => seasonTag(c) ?? `ch ${c.number}`;
