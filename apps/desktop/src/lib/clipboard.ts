import { isTauri } from "./ipc";

/**
 * Copies `text`. In the app this goes through the Tauri clipboard plugin, which works without a
 * user gesture (WebKitGTK refuses web clipboard writes after an awaited IPC call). In the browser
 * it uses the async clipboard API, then a hidden textarea where that is missing.
 */
export async function copyText(text: string): Promise<void> {
  if (isTauri()) {
    const { writeText } = await import("@tauri-apps/plugin-clipboard-manager");
    await writeText(text);
    return;
  }
  if (navigator.clipboard?.writeText) {
    try {
      await navigator.clipboard.writeText(text);
      return;
    } catch {
      // Fall through to the legacy path.
    }
  }
  const area = document.createElement("textarea");
  area.value = text;
  area.setAttribute("readonly", "");
  area.style.position = "fixed";
  area.style.opacity = "0";
  document.body.appendChild(area);
  area.select();
  const ok = document.execCommand("copy");
  area.remove();
  if (!ok) throw new Error("The clipboard is not available");
}
