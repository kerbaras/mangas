import * as TooltipPrimitive from "@radix-ui/react-tooltip";
import type { ComponentProps } from "react";
import { cn } from "@/lib/cn";

export const TooltipProvider = TooltipPrimitive.Provider;
export const Tooltip = TooltipPrimitive.Root;
export const TooltipTrigger = TooltipPrimitive.Trigger;

/** Keeps tooltips this far from the window edges; Radix flips or shifts them to fit. */
const EDGE_PADDING = 8;

export function TooltipContent({
  className,
  sideOffset = 6,
  collisionPadding = EDGE_PADDING,
  ...props
}: ComponentProps<typeof TooltipPrimitive.Content>) {
  return (
    <TooltipPrimitive.Portal>
      <TooltipPrimitive.Content
        sideOffset={sideOffset}
        collisionPadding={collisionPadding}
        className={cn(
          "z-50 max-w-[min(24rem,calc(100vw-1rem))] break-words rounded-md border border-stroke bg-surface px-2 py-1 text-caption text-ink shadow-md",
          className,
        )}
        {...props}
      />
    </TooltipPrimitive.Portal>
  );
}
