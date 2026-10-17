/** Native dialogs and shell actions; no-ops with sensible fallbacks outside Tauri. */
import { isTauri } from "./ipc";

export async function pickFolder(title: string, fallback: string): Promise<string | null> {
  if (!isTauri()) return fallback;
  const { open } = await import("@tauri-apps/plugin-dialog");
  const picked = await open({ directory: true, multiple: false, title });
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

export async function revealFolder(path: string): Promise<void> {
  if (!isTauri()) return;
  const { openPath } = await import("@tauri-apps/plugin-opener");
  await openPath(path);
}
