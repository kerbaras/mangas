/**
 * Update checks for the browser mock: each run "publishes" one chapter for two followed titles,
 * badges them until their details open, and stands in for the desktop notification.
 */
import { toast } from "sonner";
import type { Chapter, Manga } from "./ipc";
import { summarise, type TitleUpdate, type UpdatesBackend } from "./ipc.updates";

const key = (source: string, id: string) => `${source}:${id}`;
const fresh = new Map<string, Set<string>>();
const auto = new Set<string>();

/** Chapter numbers of the title still badged as new. */
export function mockNewCount(source: string, id: string, chapters: Chapter[]): number {
  const ids = fresh.get(key(source, id));
  if (!ids) return 0;
  return new Set(chapters.filter((c) => ids.has(c.id)).map((c) => c.number)).size;
}

export const mockAutoDownload = (source: string, id: string) => auto.has(key(source, id));

const wait = (ms: number) => new Promise<void>((r) => setTimeout(r, ms));

function publish(m: Manga, list: Chapter[]): Chapter {
  const last = Math.max(0, ...list.map((c) => Number(c.number)).filter(Number.isFinite));
  const number = String(Math.floor(last) + 1);
  const chapter: Chapter = {
    ...(list[0] ?? { manga_id: m.id, language: "en", volume: null, group: null }),
    id: `${m.id}-new-${number}`,
    title: null,
    number,
    local_path: null,
    exported_at: null,
    availability: { kind: "available" },
  } as Chapter;
  list.push(chapter);
  const k = key(m.source, m.id);
  fresh.set(k, (fresh.get(k) ?? new Set()).add(chapter.id));
  return chapter;
}

function notifyDesktop(title: string, body: string) {
  if (typeof Notification !== "undefined" && Notification.permission === "granted") {
    new Notification(title, { body });
  } else {
    toast.info(title, { description: `Desktop notification: ${body}` });
  }
}

export function mockUpdates(
  library: Map<string, Manga>,
  chaptersOf: (source: string, id: string) => Chapter[],
  notifications: () => boolean,
): UpdatesBackend {
  return {
    async checkUpdates(background) {
      await wait(900);
      const followed = [...library.values()].filter((m) => m.source !== "local");
      const updates: TitleUpdate[] = followed
        .filter((_, i) => i % 2 === 0)
        .slice(0, 2)
        .map((m) => ({
          source: m.source,
          manga_id: m.id,
          title: m.title,
          auto_download: mockAutoDownload(m.source, m.id),
          chapters: [publish(m, chaptersOf(m.source, m.id))],
        }));
      const summary = summarise(updates, []);
      if (background && notifications() && updates.length > 0)
        notifyDesktop(
          `${summary.chapters} new chapters in ${summary.titles} titles`,
          updates.map((u) => u.title).join(", "),
        );
      const queued = updates.filter((u) => u.auto_download).length;
      return { updates, failures: [], checked: followed.length, skipped: 0, summary, queued };
    },
    async markViewed(source, id) {
      fresh.delete(key(source, id));
      return library.has(key(source, id));
    },
    async setAutoDownload(source, id, on) {
      if (on) auto.add(key(source, id));
      else auto.delete(key(source, id));
      return library.has(key(source, id));
    },
  };
}
