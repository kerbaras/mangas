/**
 * The core's volume planning (`export::volume`) for the browser mock, and the preview labels.
 * The desktop always asks the core; this only keeps `pnpm dev:web` faithful.
 */
import { sortChapters } from "./chapters";
import type { Chapter } from "./ipc";
import type { Grouping, VolumePlan } from "./ipc.volumes";

const volumeOf = (c: Chapter) => c.volume?.trim() || null;
const num = (s: string) => Number.parseFloat(s);

function group(
  chapters: Chapter[],
  grouping: Grouping,
): { number: string | null; chapters: Chapter[] }[] {
  if (chapters.length === 0) return [];
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

const portable = (title: string) =>
  `${
    title
      .replace(/[\\/:*?"<>|]+/g, " ")
      .replace(/\s+/g, " ")
      .trim() || "Manga"
  }.epub`;

function bookTitle(series: string, number: string | null, chapters: Chapter[]) {
  if (number !== null) return `${series} v${number}`;
  const first = chapters[0].number;
  const last = chapters[chapters.length - 1].number;
  return first === last ? `${series} c${first}` : `${series} c${first}-${last}`;
}

export function mockPlan(series: string, chapters: Chapter[], grouping: Grouping, title?: string) {
  const books = group(sortChapters(chapters.filter((c) => c.local_path)), grouping);
  return books.map(({ number, chapters: list }): VolumePlan => {
    const name =
      books.length === 1 && title?.trim() ? title.trim() : bookTitle(series, number, list);
    return { number, title: name, file_name: portable(name), chapters: list };
  });
}

/** "Chapters 1–7 · 7 chapters", "Chapter 12". */
export function planSummary(plan: VolumePlan): string {
  const { chapters } = plan;
  if (chapters.length === 1) return `Chapter ${chapters[0].number}`;
  const range = `${chapters[0].number}–${chapters[chapters.length - 1].number}`;
  return `Chapters ${range} · ${chapters.length} chapters`;
}
