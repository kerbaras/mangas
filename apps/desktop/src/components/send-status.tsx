import { RefreshCw, Usb } from "lucide-react";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Progress } from "@/components/ui/progress";
import type { SendState } from "@/hooks/use-send";
import { megabytes } from "@/lib/send-labels";

/** Where a delivery is: looking, copying with progress, Device not found or failed + Retry, done. */
export function SendStatus({ state, onRetry }: { state: SendState; onRetry: () => void }) {
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
            <Button size="sm" onClick={onRetry} className="mt-1">
              <RefreshCw /> Retry
            </Button>
          </AlertDescription>
        </Alert>
      );
    case "done":
      return (
        <p role="status" className="whitespace-pre-line text-caption text-ink-2">
          {state.message}
        </p>
      );
    case "error":
      return (
        <>
          {state.sent && (
            <p role="status" className="whitespace-pre-line text-caption text-ink-2">
              {state.sent}
            </p>
          )}
          <Alert variant="destructive">
            <AlertTitle>{state.via === "email" ? "Email not sent" : "Copy failed"}</AlertTitle>
            <AlertDescription>
              {state.message}
              <Button size="sm" onClick={onRetry} className="mt-1">
                <RefreshCw /> Retry
              </Button>
            </AlertDescription>
          </Alert>
        </>
      );
  }
}
