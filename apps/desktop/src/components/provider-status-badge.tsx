import { Hint } from "@/components/hint";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/cn";
import { describeFailure } from "@/lib/errors";
import type { ProviderStatus } from "@/lib/ipc";
import { statusBadge, type Tone } from "@/lib/providers";

// No spinner while pending: connecting waits on the user, and an endless spin keeps the webview
// repainting the whole time.
const dot: Record<Tone, string> = {
  pending: "bg-ink-3",
  ok: "bg-ok",
  warn: "bg-warn",
  bad: "bg-bad",
};

type Props = { status: ProviderStatus | undefined; label?: string; className?: string };

/** Ready / Needs connection / Unreachable with a coloured dot; a grey one while checking. */
export function ProviderStatusBadge({ status, label, className }: Props) {
  const badge = statusBadge(status);
  const reason =
    status?.state === "unreachable" ? describeFailure(status.reason).tooltip : undefined;
  return (
    <Hint hint={reason}>
      <Badge variant="outline" className={cn("font-normal", className)}>
        <span aria-hidden className={cn("size-1.5 rounded-full", dot[badge.tone])} />
        {label ?? badge.label}
      </Badge>
    </Hint>
  );
}
