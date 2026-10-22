/**
 * The core's volume planning (`export::volume`) for the browser mock, and the preview labels.
 * The desktop always asks the core; this only keeps `pnpm dev:web` faithful.
 */
import { sortChapters } from "./chapters";
import type { Chapter } from "./ipc";
import type { Grouping, VolumePlan } from "./ipc.volumes";
import { seasonTag } from "./season-tag";

const volumeOf = (c: Chapter) => c.volume?.trim() || null;
const num = (s: string) => Number.parseFloat(s);

function group(
  chapters: Chapter[],
  grouping: Grouping,
): { number: string | null; chapters: Chapter[] }[] {
  if (chapters.length === 0) return [];
  if (grouping.kind === "per_chapter")
    return chapters.map((c) => ({ number: null, chapters: [c] }));
  if (grouping.kind === "every") {
    const size = Math.max(1, grouping.size);
    const out = [];
    for (let i = 0; i < chapters.length; i += size)
      out.push({ number: String(i / size + 1), chapters: chapters.slice(i, i + size) });
    return out;
  }
  if (grouping.kind === "single") {
    const first = volumeOf(chapters[0]);
    const shared = chapters.every((c) => volumeOf(c) === first) ? first : null;
    return [{ number: shared, chapters }];
  }
  const byVolume = new Map<string, Chapter[]>();
  const loose: Chapter[] = [];
  for (const c of chapters) {
    const v = volumeOf(c);
    if (v === null) loose.push(c);
    else byVolume.set(v, [...(byVolume.get(v) ?? []), c]);
  }
  const volumes = [...byVolume.entries()]
    .sort(([a], [b]) => num(a) - num(b))
    .map(([number, list]) => ({ number: number as string | null, chapters: list }));
  return loose.length ? [...volumes, { number: null, chapters: loose }] : volumes;
}

const bytes = (s: string) => new TextEncoder().encode(s).length;
const clean = (s: string) =>
  s
    .replace(/[\\/:*?"<>|]+/g, " ")
    .replace(/\s+/g, " ")
    .trim();

/** The longest start of `s` of at most `max` UTF-8 bytes, whole characters only. */
function prefix(s: string, max: number): string {
  let out = "";
  for (const c of s) {
    if (bytes(out + c) > max) break;
    out += c;
  }
  return out.trimEnd();
}

/**
 * Mirrors `volume::series_file_name`: a stem over 200 bytes is cut at a character and marked
 * "…", inside `series` when the title holds it (so `v3` / `ch 1-5` stays), else at the end.
 */
export function portable(title: string, series = ""): string {
  const stem = clean(title) || "Manga";
  if (bytes(stem) <= 200) return `${stem}.epub`;
  const s = clean(series);
  const at = s ? stem.indexOf(s) : -1;
  const rest = bytes(stem) - bytes(s);
  const cut =
    at >= 0 && rest + 3 < 200
      ? `${stem.slice(0, at)}${prefix(s, 200 - rest - 3)}…${stem.slice(at + s.length)}`
      : `${prefix(stem, 197)}…`;
  return `${cut.replace(/[. ]+$/, "")}.epub`;
}

const tag = (c: Chapter) => seasonTag(c) ?? `ch ${c.number}`;

function bookTitle(series: string, number: string | null, chapters: Chapter[]) {
  if (number !== null) return `${series} v${number}`;
  const [first, last] = [chapters[0], chapters[chapters.length - 1]];
  if (tag(first) === tag(last)) return `${series} ${tag(first)}`;
  if (seasonTag(first) || seasonTag(last)) return `${series} ${tag(first)} - ${tag(last)}`;
  return `${series} ch ${first.number}-${last.number}`;
}

export function mockPlan(
  series: string,
  chapters: Chapter[],
  grouping: Grouping,
  title?: string,
  authors: string[] = [],
) {
  const books = group(sortChapters(chapters.filter((c) => c.local_path)), grouping);
  return books.map(({ number, chapters: list }): VolumePlan => {
    const own = books.length === 1 ? title?.trim() : undefined;
    const name = own || bookTitle(series, number, list);
    const file_name = portable(name, own ? "" : series);
    return { number, title: name, file_name, chapters: list, authors };
  });
}

/** "Chapters 1–7 · 7 chapters", "Chapter 12". */
export function planSummary(plan: VolumePlan): string {
  const { chapters } = plan;
  if (chapters.length === 1) return `Chapter ${chapters[0].number}`;
  const range = `${chapters[0].number}–${chapters[chapters.length - 1].number}`;
  return `Chapters ${range} · ${chapters.length} chapters`;
}
