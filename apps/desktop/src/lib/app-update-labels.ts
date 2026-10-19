import { describeFailure } from "./errors";
/** Wording for app updates; the CLI's `Status::line` says the same. */
import type { AppUpdateChannel, AppUpdateStatus, InstallProgress } from "./ipc.app-update";

export const channelLabel = (c: AppUpdateChannel) => (c === "rc" ? "release candidates" : "stable");

export function statusLine(s: AppUpdateStatus): string {
  switch (s.kind) {
    case "up_to_date":
      return `You're up to date (v${s.current}, ${channelLabel(s.channel)} channel)`;
    case "available":
      return `Mangas v${s.version} is available (you have v${s.current})`;
    case "older":
      return `The server offers v${s.version}, older than your v${s.current}: nothing to install`;
    case "invalid_signature":
      return `v${s.version} isn't signed with the key this app trusts, so it won't be installed`;
  }
}

export const checkFailedLine = (error: string) =>
  `Couldn't check for updates: ${describeFailure(error).label}`;

const megabytes = (bytes: number) => `${(bytes / 1_000_000).toFixed(1)} MB`;

/** Percent done, or null while the size is unknown. */
export function progressPercent(p: InstallProgress | null): number | null {
  if (p?.kind === "installing") return 100;
  if (p?.kind !== "downloading" || !p.total) return null;
  return Math.min(100, Math.floor((100 * p.received) / p.total));
}

export function progressLine(p: InstallProgress | null): string {
  if (p === null) return "Starting download…";
  if (p.kind === "installing") return "Installing, the app restarts in a moment…";
  const percent = progressPercent(p);
  const size = p.total ? ` of ${megabytes(p.total)}` : "";
  return percent === null
    ? `Downloading… ${megabytes(p.received)}`
    : `Downloading… ${percent}%${size}`;
}
