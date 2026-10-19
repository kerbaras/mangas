/**
 * App updates for the browser mock: up to date unless `?update` is set; `?update-fail` (or
 * `?offline`) fails the check, `?install-fail` fails the download after it reaches 100%.
 */
import type { AppUpdateBackend } from "./ipc.app-update";
import { mockFlags } from "./mock-flags";

const CURRENT = "0.4.0";
const TOTAL = 18_400_000;
const wait = (ms: number) => new Promise((r) => setTimeout(r, ms));

export const mockAppUpdate = (): AppUpdateBackend => ({
  async checkAppUpdate(channel) {
    await wait(500);
    if (mockFlags.updateFail || mockFlags.offline)
      throw new Error("error sending request for url (https://github.com/kerbaras/mangas/…)");
    if (!mockFlags.update) return { kind: "up_to_date", current: CURRENT, channel };
    const version = channel === "rc" ? "0.5.0-rc.2" : "0.5.0";
    return { kind: "available", current: CURRENT, version, notes: null, date: null, channel };
  },
  async installAppUpdate(_version, onProgress) {
    for (let received = 0; received < TOTAL; received += 1_300_000) {
      onProgress({ kind: "downloading", received, total: TOTAL });
      await wait(150);
    }
    onProgress({ kind: "downloading", received: TOTAL, total: TOTAL });
    if (mockFlags.installFail) throw new Error("the signature of the update doesn't match");
    onProgress({ kind: "installing" });
    await wait(800);
    window.location.reload();
  },
});
