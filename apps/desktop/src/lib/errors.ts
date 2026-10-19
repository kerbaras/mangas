import { isChallenge } from "./ipc";
import { NO_ROOT_CERTIFICATES } from "./ipc.downloads";

export type ErrorInfo = {
  /** A sentence a user can act on, when the error kind is known. */
  hint: string | null;
  /** The raw message, as logged. */
  detail: string;
  challenge: boolean;
};

/** Core error prefixes (`mangas_core::Error` display strings) and what they mean for a user. */
const HINTS: [prefix: string, hint: string][] = [
  ["network:", "Couldn't reach the site. Check your connection and try again."],
  ["source ", "The site answered with something unexpected. It may be down or have changed."],
  ["library:", "The library database couldn't be read or written."],
  ["io:", "A file couldn't be read or written. Check the data folder in Settings."],
  ["image:", "A page image couldn't be decoded."],
  ["not found:", "It no longer exists on the site."],
  ["refused:", "The image host points at a local or private address, so the app won't fetch it."],
];

/** Splits an IPC error message into a readable hint and the raw detail. */
export function describeError(message: string): ErrorInfo {
  const detail = message.trim() || "Unknown error";
  if (isChallenge(detail))
    return {
      hint: "This site asks for a browser check. Connect once and the app reuses the session.",
      detail,
      challenge: true,
    };
  if (detail.includes(NO_ROOT_CERTIFICATES))
    return {
      hint: "This system has no CA certificates, so secure connections can't be checked. Install the ca-certificates package.",
      detail,
      challenge: false,
    };
  const lower = detail.toLowerCase();
  const match = HINTS.find(([prefix]) => lower.startsWith(prefix));
  return { hint: match?.[1] ?? null, detail, challenge: false };
}

/** A failure for a short label and its tooltip: the readable hint when known, else the raw text. */
export function describeFailure(message: string): { label: string; tooltip: string } {
  const { hint, detail } = describeError(message);
  return hint
    ? { label: hint, tooltip: `${hint} (${detail})` }
    : { label: detail, tooltip: detail };
}

/** Text for "Copy error" and the log: what crashed and where. */
export function crashReport(error: Error, componentStack?: string | null): string {
  return [
    `${error.name}: ${error.message}`,
    error.stack ?? "",
    componentStack ? `Component stack:${componentStack}` : "",
  ]
    .filter(Boolean)
    .join("\n\n");
}
