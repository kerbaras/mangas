/** Words for Send to Kindle states and history rows. Pure. */
import type { ExportRecord } from "./ipc.send";

/** The EPUB's file name without folders or extension: the title a delivery records. */
export const titleOf = (file: string): string =>
  (file.split(/[\\/]/).pop() ?? file).replace(/\.epub$/i, "");

export const fileName = (path: string): string => path.split(/[\\/]/).pop() ?? path;

export const megabytes = (bytes: number): string => `${(bytes / 1e6).toFixed(1)} MB`;

/** "Emailed to reader@kindle.com · 2026-10-18 09:12", "Copied to Kindle · …", or "Not sent". */
export function deliveryLabel(r: ExportRecord): string {
  if (!r.sent_via || !r.sent_at) return "Not sent";
  const verb = r.sent_via === "email" ? "Emailed to" : "Copied to";
  return `${verb} ${r.sent_to ?? ""} · ${r.sent_at.slice(0, 16)} UTC`;
}

/** Whether the Kindle address, sender and server are filled in. */
export const emailReady = (s: { to: string; from: string; username: string; host: string }) =>
  Boolean(s.to.trim() && s.host.trim() && (s.from.trim() || s.username.trim()));

/** Why Amazon's email service won't take this file, or null when it will (EPUB and PDF). */
export function emailRefusal(file: string): string | null {
  if (/\.(epub|pdf)$/i.test(file)) return null;
  const ext = file.match(/\.([^./\\]+)$/)?.[1]?.toUpperCase();
  const what = ext ? `a ${ext}` : "this file";
  return `Send to Kindle by email takes EPUB or PDF, not ${what}. Copy it over USB, or export it as EPUB.`;
}
