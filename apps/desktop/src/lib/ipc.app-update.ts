/** App self-update contract: `commands/app_update.rs` over `mangas_core::app_update`. */
export type AppUpdateChannel = "stable" | "rc";

export type AppUpdateStatus =
  | { kind: "up_to_date"; current: string; channel: AppUpdateChannel }
  | {
      kind: "available";
      current: string;
      version: string;
      notes: string | null;
      date: string | null;
      channel: AppUpdateChannel;
    }
  /** The server offers an older version: nothing to install. */
  | { kind: "older"; current: string; version: string; channel: AppUpdateChannel }
  /** Newer, but not signed with the key this app trusts: never offered. */
  | { kind: "invalid_signature"; current: string; version: string; channel: AppUpdateChannel };

export type InstallProgress =
  | { kind: "downloading"; received: number; total: number | null }
  | { kind: "installing" };

export type AppUpdateBackend = {
  checkAppUpdate(channel: AppUpdateChannel): Promise<AppUpdateStatus>;
  /** Downloads and installs `version`, found by the last check, then relaunches the app. */
  installAppUpdate(version: string, onProgress: (p: InstallProgress) => void): Promise<void>;
};
