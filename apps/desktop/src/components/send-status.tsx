import { RefreshCw, Usb } from "lucide-react";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Progress } from "@/components/ui/progress";
import { refused, type SendState, type Via } from "@/hooks/use-send";
import { megabytes } from "@/lib/send-labels";

type Props = {
  state: SendState;
  /** The delivery picked now: a failure of another one is no longer what Retry does. */
  via?: Via | "save";
  onRetry?: () => void;
};

/** Where a delivery is: looking, copying with progress, Device not found or failed (+ Retry when given), done. */
export function SendStatus({ state, via, onRetry }: Props) {
  const failed = failedVia(state);
  if (via !== undefined && failed !== undefined && failed !== via) {
    return state.kind === "error" && state.sent ? <Sent lines={state.sent} /> : null;
  }
  switch (state.kind) {
    case "idle":
      return null;
    case "looking":
      return <p className="text-caption text-ink-3">Looking for a Kindle…</p>;
    case "emailing":
      return <p className="text-caption text-ink-3">Sending email…</p>;
    case "copying":
      return (
        <div aria-live="polite">
          <div className="flex justify-between text-caption text-ink-3">
            <span>Copying to {state.device}…</span>
            {state.total > 0 && (
              <span className="tabular-nums">
                {megabytes(state.done)} / {megabytes(state.total)}
              </span>
            )}
          </div>
          <Progress value={state.done} max={state.total} className="mt-1.5" />
        </div>
      );
    case "no-device":
      return (
        <Alert>
          <Usb />
          <AlertTitle>Device not found</AlertTitle>
          <AlertDescription>
            Connect your Kindle with a USB cable and unlock it; it shows up as a drive named Kindle.
            {onRetry && (
              <Button size="sm" onClick={onRetry} className="mt-1">
                <RefreshCw /> Retry
              </Button>
            )}
          </AlertDescription>
        </Alert>
      );
    case "done":
      return (
        <p
          role="status"
          className="whitespace-pre-line text-caption text-ink-2 [overflow-wrap:anywhere]"
        >
          {state.message}
        </p>
      );
    case "error":
      return (
        <>
          {state.sent && <Sent lines={state.sent} />}
          <Alert variant="destructive">
            <AlertTitle>{state.via === "email" ? "Email not sent" : "Copy failed"}</AlertTitle>
            <AlertDescription>
              {state.message}
              {state.left && (state.sent || state.left.length > 1) && (
                <p className="[overflow-wrap:anywhere]">Not sent: {state.left.join(", ")}</p>
              )}
              {onRetry && !refused(state, state.via) && (
                <Button size="sm" onClick={onRetry} className="mt-1">
                  <RefreshCw /> Retry
                </Button>
              )}
            </AlertDescription>
          </Alert>
        </>
      );
  }
}

/** The delivery a failed `state` was for, or `undefined` when nothing failed. */
function failedVia(state: SendState): Via | undefined {
  if (state.kind === "error") return state.via;
  if (state.kind === "no-device") return "usb";
  return undefined;
}

function Sent({ lines }: { lines: string }) {
  return (
    <p
      role="status"
      className="whitespace-pre-line text-caption text-ink-2 [overflow-wrap:anywhere]"
    >
      {lines}
    </p>
  );
}
