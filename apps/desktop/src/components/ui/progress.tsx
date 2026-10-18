import * as ProgressPrimitive from "@radix-ui/react-progress";
import type { ComponentProps } from "react";
import { cn } from "@/lib/cn";

type Props = ComponentProps<typeof ProgressPrimitive.Root> & { value: number; max?: number };

/** Indeterminate (no value, pulsing third) while `max` is not known yet (0). */
export function Progress({ className, value, max = 100, ...props }: Props) {
  const known = max > 0;
  const pct = known ? Math.min(100, Math.max(0, (value / max) * 100)) : 33;
  return (
    <ProgressPrimitive.Root
      value={known ? value : null}
      max={known ? max : undefined}
      className={cn("relative h-1 w-full overflow-hidden rounded-full bg-surface-2", className)}
      {...props}
    >
      <ProgressPrimitive.Indicator
        className={cn(
          "h-full bg-accent transition-[width] duration-200",
          !known && "animate-pulse",
        )}
        style={{ width: `${pct}%` }}
      />
    </ProgressPrimitive.Root>
  );
}
