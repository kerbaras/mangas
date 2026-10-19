/**
 * Browser stand-in for the Send to Kindle commands. `?no-kindle` hides the USB Kindle until
 * the second look (Retry then finds it); `?smtp-fail` makes every email fail as a wrong
 * password would; `?no-keychain` keeps the password in the settings file, `?locked-keychain`
 * reports a keychain that won't open.
 */
import type { Unlisten } from "./ipc";
import type { CopyProgress, ExportRecord, SendBackend } from "./ipc.send";

const wait = (ms: number) => new Promise((r) => setTimeout(r, ms));
const params = new URLSearchParams(typeof window === "undefined" ? "" : window.location.search);
const KINDLE = {
  name: "Kindle",
  root: "/media/me/Kindle",
  documents: "/media/me/Kindle/documents",
};
const SMTP_FAIL =
  "The mail server refused the user name or password. Gmail, iCloud and Outlook need an app password here (535 5.7.8 Username and Password not accepted)";

const now = () => new Date().toISOString().slice(0, 19).replace("T", " ");

export function mockSend(): SendBackend {
  let looks = 0;
  const listeners = new Set<(p: CopyProgress) => void>();
  const history: ExportRecord[] = [];
  const stamp = (file: string, title: string, via: "email" | "usb", to: string) => {
    const row = history.find((r) => r.file === file);
    if (row) Object.assign(row, { sent_via: via, sent_to: to, sent_at: now() });
    else
      history.unshift({
        id: history.length + 1,
        file,
        title,
        device: "",
        exported_at: null,
        sent_via: via,
        sent_to: to,
        sent_at: now(),
        exists: true,
      });
  };
  return {
    async findKindles() {
      await wait(300);
      looks += 1;
      return params.has("no-kindle") && looks < 2 ? [] : [KINDLE];
    },
    async sendUsb(file, title) {
      const path = `${KINDLE.documents}/${file.split("/").pop()}`;
      if (file === path) return { path, already_there: true };
      const total = 18_400_000;
      for (let step = 0; step <= 10; step++) {
        for (const cb of listeners) cb({ done: (total * step) / 10, total });
        await wait(120);
      }
      stamp(file, title, "usb", KINDLE.name);
      return { path, already_there: false };
    },
    onCopyProgress: async (cb): Promise<Unlisten> => {
      listeners.add(cb);
      return () => listeners.delete(cb);
    },
    async sendEmail(file, title) {
      await wait(900);
      if (params.has("smtp-fail")) throw new Error(SMTP_FAIL);
      stamp(file, title, "email", "reader@kindle.com");
      return { to: "reader@kindle.com", bytes: 18_400_000 };
    },
    async sendTestEmail(settings) {
      await wait(700);
      if (params.has("smtp-fail")) throw new Error(SMTP_FAIL);
      return settings.from || settings.username;
    },
    async emailPasswordStorage() {
      if (params.has("locked-keychain")) return "locked";
      return params.has("no-keychain") ? "file" : "keychain";
    },
    async listExports(limit) {
      await wait(100);
      return history.slice(0, limit);
    },
  };
}
