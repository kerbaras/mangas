/** Toasts for failures. Challenge errors get a Connect action that retries once connected. */
import { toast } from "sonner";
import { describeError } from "./errors";
import { errorMessage } from "./format";
import { connectAndWait } from "./providers";

type Options = {
  /** Provider the failing call went to; enables Connect on challenge errors. */
  source?: string;
  /** Rerun the failed action (after connecting, or from a Retry button). */
  retry?: () => void;
  /** Same id replaces the previous toast instead of stacking a new one. */
  id?: string;
};

function Description({ hint, detail }: { hint: string | null; detail: string }) {
  return (
    <span className="flex flex-col gap-0.5">
      {hint && <span>{hint}</span>}
      <span className="break-words text-ink-3">{detail}</span>
    </span>
  );
}

/** Opens the Connect window for `source`, reporting the outcome, then calls `then`. */
export async function connectThen(source: string, then?: () => void) {
  const id = `connect-${source}`;
  toast.loading(`Pass the check in the ${source} window`, {
    id,
    description: "It closes by itself when done.",
  });
  try {
    if (await connectAndWait(source)) {
      toast.success(`Connected to ${source}`, { id, description: null });
      then?.();
    } else {
      toast.error(`No session captured for ${source}`, {
        id,
        description: "Close its window and try again.",
      });
    }
  } catch (e: unknown) {
    toast.error(`Couldn't connect to ${source}`, { id, description: errorMessage(e) });
  }
}

export function notifyError(title: string, error: unknown, { source, retry, id }: Options = {}) {
  const info = describeError(errorMessage(error));
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
    action: retry ? { label: "Retry", onClick: retry } : undefined,
  });
}
