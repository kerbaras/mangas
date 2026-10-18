/** Diagnostics part of the IPC contract: log folder, shareable report, UI error logging. */
import { backend } from "./ipc";
import { openFolder } from "./ipc.folders";

/** Settings → Diagnostics: the log folder and the plain-text report. */
export type Diagnostics = { log_dir: string; report: string };

/** Log level for UI messages: crashes are errors, recoverable failures warnings. */
export type UiLogLevel = "error" | "warn";

export type DiagnosticsBackend = {
  diagnostics(): Promise<Diagnostics>;
  /** Writes a UI error (crash, unhandled rejection) or warning to the app log; error by default. */
  reportError(message: string, level?: UiLogLevel): Promise<void>;
};

export const diagnostics = async () => (await backend()).diagnostics();
/** Opens the log folder the diagnostics report names. */
export const openLogs = async () => openFolder((await diagnostics()).log_dir);
export const reportError = async (message: string, level?: UiLogLevel) => {
  const b = await backend();
  return level ? b.reportError(message, level) : b.reportError(message);
};
