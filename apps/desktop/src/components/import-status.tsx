import { X } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { ImportStatus } from "@/hooks/use-import";
import { cn } from "@/lib/cn";

type Props = { status: ImportStatus; onDismiss: () => void };

/** Inline result of the last import, under the header; dismissable. */
export function ImportStatusBar({ status, onDismiss }: Props) {
  if (status.kind === "idle") return null;
  const text =
    status.kind === "busy"
      ? "Importing…"
      : status.kind === "error"
        ? `Couldn't import: ${status.message}`
        : status.message;
  return (
    <div
      role="status"
      className={cn(
        "flex h-9 shrink-0 items-center gap-2 border-b border-stroke px-4 text-caption",
        status.kind === "error" ? "text-accent" : "text-ink-2",
      )}
    >
      <span className="truncate">{text}</span>
      {status.kind !== "busy" && (
        <Button
          variant="ghost"
          size="icon"
          aria-label="Dismiss"
          onClick={onDismiss}
          className="ml-auto size-6"
        >
          <X />
        </Button>
      )}
    </div>
  );
}
