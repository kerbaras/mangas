import { isDownloaded, sortChapters } from "@/lib/chapters";
import { type Chapter, listChapters, listLibrary, type Manga } from "@/lib/ipc";
import { useAsync } from "./use-async";

export type DownloadedGroup = { manga: Manga; chapters: Chapter[] };

async function loadDownloaded(): Promise<DownloadedGroup[]> {
  const library = await listLibrary();
  const groups = await Promise.all(
    library.map(async (manga) => ({
      manga,
      chapters: sortChapters((await listChapters(manga.source, manga.id)).filter(isDownloaded)),
    })),
  );
  return groups.filter((g) => g.chapters.length > 0);
}

/** Everything on disk, grouped by manga, for the export picker. */
export const useDownloadedChapters = () => useAsync(loadDownloaded, []);
