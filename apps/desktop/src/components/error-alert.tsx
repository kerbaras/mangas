import { AlertCircle, RotateCw } from "lucide-react";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { describeError } from "@/lib/errors";

export function RetryButton({ onClick, className }: { onClick: () => void; className?: string }) {
  return (
    <Button size="sm" className={className} onClick={onClick}>
      <RotateCw /> Try again
    </Button>
  );
}

type Props = { title: string; message: string; onRetry?: () => void; className?: string };

/** Inline failure with the readable hint, the raw message and an optional retry. */
export function ErrorAlert({ title, message, onRetry, className }: Props) {
  const { hint, detail } = describeError(message);
  return (
    <Alert variant="destructive" className={className}>
      <AlertCircle />
      <AlertTitle>{title}</AlertTitle>
      <AlertDescription>
        {hint && <p>{hint}</p>}
        <p className="break-all text-caption text-ink-3">{detail}</p>
        {onRetry && <RetryButton onClick={onRetry} className="mt-2" />}
      </AlertDescription>
    </Alert>
  );
}
