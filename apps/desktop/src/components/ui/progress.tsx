import * as ProgressPrimitive from "@radix-ui/react-progress";
import type { ComponentProps } from "react";
import { cn } from "@/lib/cn";

type Props = ComponentProps<typeof ProgressPrimitive.Root> & { value: number; max?: number };

export function Progress({ className, value, max = 100, ...props }: Props) {
  const pct = max > 0 ? Math.min(100, Math.max(0, (value / max) * 100)) : 0;
  return (
    <ProgressPrimitive.Root
      value={value}
      max={max}
      className={cn("relative h-1 w-full overflow-hidden rounded-full bg-surface-2", className)}
      {...props}
    >
      <ProgressPrimitive.Indicator
        className="h-full bg-accent transition-[width] duration-200"
        style={{ width: `${pct}%` }}
      />
    </ProgressPrimitive.Root>
  );
}
