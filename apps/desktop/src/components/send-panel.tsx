import { Mail, RefreshCw, Usb } from "lucide-react";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Progress } from "@/components/ui/progress";
import { type SendState, useSend } from "@/hooks/use-send";
import { useNavigation } from "@/lib/navigation";
import { emailReady, fileName, megabytes, titleOf } from "@/lib/send-labels";
import { useSettings } from "@/lib/settings";

type Props = { file: string; onSent?: () => void };

/** Email or USB delivery for one EPUB, with copy progress and a "Device not found" state. */
export function SendPanel({ file, onSent }: Props) {
  const [settings] = useSettings();
  const nav = useNavigation();
  const send = useSend(onSent);
  const busy = ["looking", "copying", "emailing"].includes(send.state.kind);
  const title = titleOf(file);
  const canEmail = emailReady(settings.kindleEmail);

  return (
    <div className="flex flex-col gap-3 rounded-md border border-stroke bg-surface p-3">
      <div className="flex items-center justify-between gap-3">
        <p className="min-w-0 truncate text-body font-medium" title={file}>
          Send {fileName(file)}
        </p>
        <div className="flex shrink-0 gap-2">
          <Button
            size="sm"
            disabled={busy || !canEmail}
            onClick={() => void send.email(file, title)}
          >
            <Mail /> Email to Kindle
          </Button>
          <Button size="sm" disabled={busy} onClick={() => void send.usb(file, title)}>
            <Usb /> Copy over USB
          </Button>
        </div>
      </div>
      {!canEmail && (
        <p className="text-caption text-ink-3">
          Email needs your Kindle address and a mail server.{" "}
          <button
            type="button"
            className="text-accent underline-offset-2 hover:underline"
            onClick={() => nav.go({ screen: "settings" })}
          >
            Set up email
          </button>
        </p>
      )}
      <SendStatus state={send.state} onRetry={() => void send.usb(file, title)} />
    </div>
  );
}

function SendStatus({ state, onRetry }: { state: SendState; onRetry: () => void }) {
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
        <p role="status" className="text-caption text-ink-2">
          {state.message}
        </p>
      );
    case "error":
      return (
        <Alert variant="destructive">
          <AlertTitle>{state.via === "email" ? "Email not sent" : "Copy failed"}</AlertTitle>
          <AlertDescription>{state.message}</AlertDescription>
        </Alert>
      );
  }
}
