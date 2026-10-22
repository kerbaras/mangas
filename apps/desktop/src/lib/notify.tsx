/** Toasts for failures. Challenge errors get a Connect action that retries once connected. */

import { toast } from "sonner";
import { Hint } from "@/components/hint";
import { describeError } from "./errors";
import { errorMessage } from "./format";
import type { Folders } from "./io-hint";
import { isFinalError, REFUSED } from "./ipc.downloads";
import { connectAndWait } from "./providers";

/**
 * Sonner keeps a dismissed toast mounted this long; a same-id toast raised meanwhile (a retry that
 * fails at once) would be swallowed by the exit, so Retry waits it out.
 */
export const TOAST_EXIT_MS = 250;

type Options = {
  /** Provider the failing call went to; enables Connect on challenge errors. */
  source?: string;
  /** Rerun the failed action (after connecting, or from a Retry button). */
  retry?: () => void;
  /** Same id replaces the previous toast instead of stacking a new one. */
  id?: string;
  /** Folders an io error may be in, so its hint names the one that failed. */
  folders?: Folders;
};

const REFUSED_AT = new RegExp(`^(.*${REFUSED} )(\\S+)(.*)$`, "s");

/** Error text; a refused address stays on one line so it never splits mid-address. */
function DetailText({ text }: { text: string }) {
  const [, head, address, tail] = text.match(REFUSED_AT) ?? [];
  if (!address) return text;
  return (
    <>
      {head}
      <span className="whitespace-nowrap">{address}</span>
      {tail}
    </>
  );
}

/**
 * Hint over the raw error, wrapping on word boundaries (sonner sets `overflow-wrap: anywhere`).
 * The error stops at three lines by height; a `-webkit-box` line clamp can split words in WebKit.
 */
export function Description({ hint, detail }: { hint: string | null; detail: string }) {
  return (
    <span className="flex flex-col gap-0.5 break-words">
      {hint && <span>{hint}</span>}
      <Hint hint={detail}>
        <span className="max-h-12 overflow-hidden text-ink-3">
          <DetailText text={detail} />
        </span>
      </Hint>
    </span>
  );
}

/** Opens the Connect window for `source` (a provider id), reporting the outcome under `name`. */
export async function connectThen(source: string, then?: () => void, name = source) {
  const id = `connect-${source}`;
  toast.loading(`Pass the check in the ${name} window`, {
    id,
    description: "It closes by itself when done.",
  });
  try {
    const outcome = await connectAndWait(source);
    if (outcome === "connected") {
      toast.success(`Connected to ${name}`, { id, description: null });
      then?.();
    } else if (outcome === "cancelled") {
      toast.info("Connection cancelled", { id, description: null });
    } else {
      toast.error(`No session captured for ${name}`, {
        id,
        description: "Close its window and try again.",
      });
    }
  } catch (e: unknown) {
    const info = describeError(errorMessage(e));
    const description = <Description hint={info.hint} detail={info.detail} />;
    toast.error(`Couldn't connect to ${name}`, { id, description });
  }
}

export function notifyError(title: string, error: unknown, options: Options = {}) {
  const info = describeError(errorMessage(error), options.folders);
  const { source, id } = options;
  const retry = isFinalError(info.detail) ? undefined : options.retry;
  const description = <Description hint={info.hint} detail={info.detail} />;
  if (info.challenge && source) {
    toast.warning(title, {
      id,
      description,
      duration: 15_000,
      action: { label: "Connect", onClick: () => void connectThen(source, retry) },
    });
    return;
  }
  toast.error(title, {
    id,
    description,
    duration: 10_000,
    // null, not undefined: a toast this one replaces may carry another action (e.g. Connect).
    action: retry ? { label: "Retry", onClick: () => setTimeout(retry, TOAST_EXIT_MS) } : null,
  });
}
