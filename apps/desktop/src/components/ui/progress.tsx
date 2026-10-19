import * as ProgressPrimitive from "@radix-ui/react-progress";
import type { ComponentProps } from "react";
import { cn } from "@/lib/cn";

type Props = ComponentProps<typeof ProgressPrimitive.Root> & {
  value: number;
  max?: number;
  /** Off for bars that are not moving (paused, queued, stalled): no transition, no pulse. */
  animated?: boolean;
};

/**
 * Indeterminate (no value, pulsing third while `animated`) until `max` is known (not 0). The fill
 * scales instead of changing width, so a moving bar never triggers layout.
 */
export function Progress({ className, value, max = 100, animated = true, ...props }: Props) {
  const known = max > 0;
  const idle = known ? 0 : 33;
  const pct = known ? Math.min(100, Math.max(0, (value / max) * 100)) : animated ? idle : 0;
  return (
    <ProgressPrimitive.Root
      value={known ? value : null}
      max={known ? max : undefined}
      className={cn("relative h-1 w-full overflow-hidden rounded-full bg-surface-2", className)}
      {...props}
    >
      <ProgressPrimitive.Indicator
        className={cn(
          "h-full w-full origin-left bg-accent will-change-transform",
          animated && "transition-transform duration-200",
          animated && !known && "animate-pulse",
        )}
        style={{ transform: `scaleX(${pct / 100})` }}
      />
    </ProgressPrimitive.Root>
  );
}
