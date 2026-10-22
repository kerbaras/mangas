/** Update checks part of the IPC contract (`commands/updates.rs`). */
import { tr } from "@/i18n/tr";
import { backend, type Chapter } from "./ipc";

/** Mirrors `library::updates::TitleUpdate`: new chapters found for one title. */
export type TitleUpdate = {
  source: string;
  manga_id: string;
  title: string;
  auto_download: boolean;
  chapters: Chapter[];
};

/** Mirrors `library::updates::CheckFailure`: a title the run could not refresh. */
export type CheckFailure = {
  source: string;
  manga_id: string;
  title: string;
  error: string;
  challenge: boolean;
};

/** Mirrors `library::updates::Summary`. */
export type UpdateSummary = { chapters: number; titles: number; unreachable: number };

/** One run: the report with its totals. */
export type UpdateRun = {
  updates: TitleUpdate[];
  failures: CheckFailure[];
  checked: number;
  skipped: number;
  summary: UpdateSummary;
  /** Chapters queued for download (titles with auto-download on). */
  queued: number;
};

export type UpdatesBackend = {
  /** `background` runs (launch, timer) send a desktop notification when something is new. */
  checkUpdates(background: boolean): Promise<UpdateRun>;
  /** Clears the title's "new" badge; false when it isn't followed. */
  markViewed(source: string, mangaId: string): Promise<boolean>;
  setAutoDownload(source: string, mangaId: string, on: boolean): Promise<boolean>;
  /** Narrows the title to one scanlation group, `null` for every group. */
  setChapterGroup(source: string, mangaId: string, group: string | null): Promise<boolean>;
};

/** Same wording as `Summary::line`: "12 new chapters in 3 titles", "Up to date"… */
export function summaryLine({ chapters, titles, unreachable }: UpdateSummary): string {
  const found = chapters
    ? tr("updates.found", {
        chapters: tr("updates.newChapters", { count: chapters }),
        titles: tr("updates.titles", { count: titles }),
      })
    : null;
  const failed = unreachable ? tr("updates.unreachable", { count: unreachable }) : null;
  if (found && failed) return `${found} · ${failed}`;
  return found ?? failed ?? tr("updates.upToDate");
}

const numberKey = (c: Chapter) =>
  c.number.trim() === ""
    ? `#${c.id}`
    : Number.isFinite(Number(c.number))
      ? String(Number(c.number))
      : c.number.trim();

/** Same totals as `library::updates::summary`, for the browser mock. */
export function summarise(updates: TitleUpdate[], failures: CheckFailure[]): UpdateSummary {
  return {
    chapters: updates.reduce((n, u) => n + new Set(u.chapters.map(numberKey)).size, 0),
    titles: updates.length,
    unreachable: new Set(failures.map((f) => f.source)).size,
  };
}

export const checkUpdates = async (background: boolean) =>
  (await backend()).checkUpdates(background);
export const markViewed = async (source: string, mangaId: string) =>
  (await backend()).markViewed(source, mangaId);
export const setAutoDownload = async (source: string, mangaId: string, on: boolean) =>
  (await backend()).setAutoDownload(source, mangaId, on);

export const setChapterGroup = async (source: string, mangaId: string, group: string | null) =>
  (await backend()).setChapterGroup(source, mangaId, group);
