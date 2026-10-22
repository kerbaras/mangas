import { type Folders, ioHint, ioRest } from "./io-hint";
import { isChallenge } from "./ipc";
import { certificateFailure, isFinalError, NO_ROOT_CERTIFICATES } from "./ipc.downloads";
import { knownDataFolder } from "./known-folders";

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
  ["image:", "A page image couldn't be decoded."],
  ["not found:", "It no longer exists on the site."],
  ["refused:", "The image host points at a local or private address, so the app won't fetch it."],
  [
    "not enough space in",
    "Free up space on that disk or choose another folder, then export again.",
  ],
  ["not enough space on", "Delete some books from the Kindle, then send again."],
];

/** A stored refusal that a retry through the proxy can fetch: Retry is offered, so say why. */
const PROXIED_REFUSAL =
  "The image host points at a private address. Retry fetches it through the configured proxy.";

/**
 * What the hint can know besides the message: the folders an io error may have failed in (the
 * data folder by default), and whether a proxy is in use, so a stored hostname refusal reads as
 * retryable (see `isFinalError`).
 */
export type ErrorContext = Folders & { proxied?: boolean };

/**
 * Splits an IPC error message into a readable hint and the raw detail. An io error's hint names
 * the folder it failed in, or is null outside the known folders.
 */
export function describeError(message: string, context: ErrorContext = {}): ErrorInfo {
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
  const certificate = certificateFailure(detail);
  if (certificate)
    return {
      hint: `${certificate.host} answered, but its certificate couldn't be verified: ${certificate.problem}.`,
      detail,
      challenge: false,
    };
  const rest = ioRest(detail);
  if (rest !== null) {
    const known = { data: context.data ?? knownDataFolder(), export: context.export };
    return { hint: ioHint(rest, known), detail, challenge: false };
  }
  const lower = detail.toLowerCase();
  const match = HINTS.find(([prefix]) => lower.startsWith(prefix));
  const viaProxy = !!context.proxied && isFinalError(detail) && !isFinalError(detail, true);
  return { hint: viaProxy ? PROXIED_REFUSAL : (match?.[1] ?? null), detail, challenge: false };
}

/** A failure for a short label and its tooltip: the readable hint when known, else the raw text. */
export function describeFailure(
  message: string,
  proxied = false,
): { label: string; tooltip: string } {
  const { hint, detail } = describeError(message, { proxied });
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
