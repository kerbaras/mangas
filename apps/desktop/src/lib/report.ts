import { errorMessage } from "./format";
import { reportError } from "./ipc";

/** Sends a UI error to the app log; never throws, logging must not cause a second failure. */
export function logUiError(message: string) {
  reportError(message).catch(() => {});
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
