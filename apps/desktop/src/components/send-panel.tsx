import { Mail, Usb } from "lucide-react";
import { Hint } from "@/components/hint";
import { SendStatus } from "@/components/send-status";
import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { useSend } from "@/hooks/use-send";
import { useNavigation } from "@/lib/navigation";
import { emailReady, emailRefusal, fileName, titleOf } from "@/lib/send-labels";
import { useSettings } from "@/lib/settings";

type Props = { file: string; onSent?: () => void };

/** Email or USB delivery for one exported book, with copy progress and a "Device not found" state. */
export function SendPanel({ file, onSent }: Props) {
  const [settings] = useSettings();
  const nav = useNavigation();
  const send = useSend(onSent);
  const busy = ["looking", "copying", "emailing"].includes(send.state.kind);
  const title = titleOf(file);
  const refusal = emailRefusal(file);
  const canEmail = emailReady(settings.kindleEmail);
  const emailButton = (
    <Button
      size="sm"
      disabled={busy || !canEmail || refusal !== null}
      onClick={() => void send.email(file, title)}
    >
      <Mail /> Email to Kindle
    </Button>
  );

  return (
    <div className="flex flex-col gap-3 rounded-md border border-stroke bg-surface p-3">
      <div className="flex items-center justify-between gap-3">
        <Hint hint={file}>
          <p className="min-w-0 truncate text-body font-medium">Send {fileName(file)}</p>
        </Hint>
        <div className="flex shrink-0 gap-2">
          {refusal ? (
            <Tooltip>
              <TooltipTrigger asChild>
                {/* biome-ignore lint/a11y/noNoninteractiveTabindex: focus shows why Email is disabled */}
                <span tabIndex={0} className="rounded-md">
                  {emailButton}
                </span>
              </TooltipTrigger>
              <TooltipContent className="max-w-64">{refusal}</TooltipContent>
            </Tooltip>
          ) : (
            emailButton
          )}
          <Button size="sm" disabled={busy} onClick={() => void send.usb(file, title)}>
            <Usb /> Copy over USB
          </Button>
        </div>
      </div>
      {!canEmail && !refusal && (
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
