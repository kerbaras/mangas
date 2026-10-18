import { errorMessage } from "./format";
import { reportError } from "./ipc.diagnostics";

/** Sends a UI error to the app log; never throws, logging must not cause a second failure. */
export function logUiError(message: string) {
  reportError(message).catch(() => {});
}

/** A failure the user can recover from (one chapter that didn't download), logged at warn. */
export function logUiWarning(message: string) {
  reportError(message, "warn").catch(() => {});
}

/** Uncaught errors and rejections in the webview go to the log file too. */
export function installGlobalErrorLogging() {
  window.addEventListener("error", (e) =>
    logUiError(`uncaught: ${errorMessage(e.error ?? e.message)}`),
  );
  window.addEventListener("unhandledrejection", (e) =>
    logUiError(`unhandled rejection: ${errorMessage(e.reason)}`),
  );
}
