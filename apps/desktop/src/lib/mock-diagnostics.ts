/** Browser stand-in for the diagnostics commands. */
import type { ProviderInfo, Settings } from "./ipc";
import type { DiagnosticsBackend } from "./ipc.diagnostics";

const status = (p: ProviderInfo, settings: Settings) =>
  p.needs_browser_session && !settings.provider_sessions[p.id]
    ? "needs a browser session"
    : "ready";

export const mockDiagnostics = (
  dataDir: string,
  providers: ProviderInfo[],
  settings: () => Settings,
): DiagnosticsBackend => ({
  async diagnostics() {
    const lines = providers.map((p) => `  ${p.id}: ${status(p, settings())}`).join("\n");
    const report = `Mangas 0.0.0-mock\nOS: browser\nData folder: ${dataDir}\nLogs: ${dataDir}/logs\nProviders:\n${lines}\n`;
    return { log_dir: `${dataDir}/logs`, report };
  },
  async openLogs() {},
  async reportError(message, level) {
    if (level === "warn") console.warn(message);
    else console.error(message);
  },
});
