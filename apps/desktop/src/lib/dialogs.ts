/** Native dialogs and shell actions; no-ops with sensible fallbacks outside Tauri. */
import { isTauri } from "./ipc";

export async function pickFolder(title: string, fallback: string): Promise<string | null> {
  if (!isTauri()) return fallback;
  const { open } = await import("@tauri-apps/plugin-dialog");
  const picked = await open({ directory: true, multiple: false, title });
  return typeof picked === "string" ? picked : null;
}

export async function revealFolder(path: string): Promise<void> {
  if (!isTauri()) return;
  const { openPath } = await import("@tauri-apps/plugin-opener");
  await openPath(path);
}
