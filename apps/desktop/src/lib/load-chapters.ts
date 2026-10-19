import { type Chapter, listChapters } from "./ipc";
import { settingsSettled } from "./settings";

/**
 * Syncs the chapter list from the source. When that fails but the library has a stored list,
 * reports the failure through `onRefreshError` and returns the stored one; with nothing stored
 * the refresh error is rethrown. Waits for the settings to settle, like Details (#99).
 */
export async function loadChapters(
  source: string,
  mangaId: string,
  onRefreshError: (e: unknown) => void,
): Promise<Chapter[]> {
  await settingsSettled();
  try {
    return await listChapters(source, mangaId, true);
  } catch (e: unknown) {
    const stored = await listChapters(source, mangaId, false).catch(() => []);
    if (stored.length === 0) throw e;
    onRefreshError(e);
    return stored;
  }
}
