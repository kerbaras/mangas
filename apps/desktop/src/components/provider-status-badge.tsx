import { Loader2 } from "lucide-react";
import { Hint } from "@/components/hint";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/cn";
import { describeFailure } from "@/lib/errors";
import type { ProviderStatus } from "@/lib/ipc";
import { statusBadge, type Tone } from "@/lib/providers";

const dot: Record<Exclude<Tone, "pending">, string> = {
  ok: "bg-ok",
  warn: "bg-warn",
  bad: "bg-bad",
};

type Props = { status: ProviderStatus | undefined; label?: string; className?: string };

/** Ready / Needs connection / Unreachable with a coloured dot; a spinner while checking. */
export function ProviderStatusBadge({ status, label, className }: Props) {
  const badge = statusBadge(status);
  const reason =
    status?.state === "unreachable" ? describeFailure(status.reason).tooltip : undefined;
  return (
    <Hint hint={reason}>
      <Badge variant="outline" className={cn("font-normal", className)}>
        {badge.tone === "pending" ? (
          <Loader2 aria-hidden className="size-3 animate-spin text-ink-3" />
        ) : (
          <span aria-hidden className={cn("size-1.5 rounded-full", dot[badge.tone])} />
        )}
        {label ?? badge.label}
      </Badge>
    </Hint>
  );
}
