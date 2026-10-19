import { isTauri } from "./ipc";

/** The app's version; "dev" in the browser preview. */
export async function appVersion(): Promise<string> {
  if (!isTauri()) return "dev";
  const { getVersion } = await import("@tauri-apps/api/app");
  return getVersion();
}
