/** Native dialogs and shell actions; no-ops with sensible fallbacks outside Tauri. */
import { isTauri } from "./ipc";
import { openFolder } from "./ipc.folders";
import { notifyError } from "./notify";

export async function pickFolder(title: string, fallback: string): Promise<string | null> {
  if (!isTauri()) return fallback;
  const { open } = await import("@tauri-apps/plugin-dialog");
  const picked = await open({ directory: true, multiple: false, title });
  return typeof picked === "string" ? picked : null;
}

const BACKUP_FILTERS = [{ name: "Library backup", extensions: ["json", "bookmarks"] }];

/** Where to save a library backup; null when cancelled. */
export async function pickBackupTarget(): Promise<string | null> {
  const name = `mangas-backup-${new Date().toISOString().slice(0, 10)}.json`;
  if (!isTauri()) return `~/${name}`;
  const { save } = await import("@tauri-apps/plugin-dialog");
  return save({ title: "Export library backup", defaultPath: name, filters: BACKUP_FILTERS });
}

/** A mangas backup or HaruNeko bookmarks file to import; null when cancelled. */
export async function pickBackupFile(): Promise<string | null> {
  if (!isTauri()) return "~/mangas-backup.json";
  const { open } = await import("@tauri-apps/plugin-dialog");
  const picked = await open({
    multiple: false,
    title: "Import library backup",
    filters: BACKUP_FILTERS,
  });
  return typeof picked === "string" ? picked : null;
}

export type ImportKind = "folder" | "cbz";

/** A series folder, or several `.cbz` files; empty when cancelled. */
export async function pickImportPaths(kind: ImportKind): Promise<string[]> {
  if (!isTauri()) return [kind === "folder" ? "~/Manga/Blame!" : "~/Manga/Blame!/Blame! c001.cbz"];
  const { open } = await import("@tauri-apps/plugin-dialog");
  const picked =
    kind === "folder"
      ? await open({ directory: true, multiple: false, title: "Import a series folder" })
      : await open({
          multiple: true,
          title: "Import CBZ files",
          filters: [{ name: "Comic book archive", extensions: ["cbz", "zip"] }],
        });
  if (picked === null) return [];
  return Array.isArray(picked) ? picked : [picked];
}

/** Opens `path` in the file manager; says so in a toast when it can't. */
export async function revealFolder(path: string): Promise<void> {
  await openFolder(path).catch((e: unknown) => notifyError("Couldn't open the folder", e));
}

/** Opens a web page in the default browser (a new tab outside Tauri). */
export async function openExternal(url: string): Promise<void> {
  if (!isTauri()) {
    window.open(url, "_blank", "noopener");
    return;
  }
  const { openUrl } = await import("@tauri-apps/plugin-opener");
  await openUrl(url);
}
