/** Diagnostics part of the IPC contract: log folder, shareable report, UI error logging. */
import { backend } from "./ipc";

/** Settings → Diagnostics: the log folder and the plain-text report. */
export type Diagnostics = { log_dir: string; report: string };

export type DiagnosticsBackend = {
  diagnostics(): Promise<Diagnostics>;
  openLogs(): Promise<void>;
  /** Writes a UI error (crash, unhandled rejection) to the app log. */
  reportError(message: string): Promise<void>;
};

export const diagnostics = async () => (await backend()).diagnostics();
export const openLogs = async () => (await backend()).openLogs();
export const reportError = async (message: string) => (await backend()).reportError(message);
