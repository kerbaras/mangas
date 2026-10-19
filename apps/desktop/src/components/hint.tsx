import { type ReactElement, useRef, useState } from "react";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { useClipped } from "@/lib/use-clipped";

type Props = {
  hint?: string | null;
  children: ReactElement;
  /** Make the trigger a tab stop. Off when `children` is, or sits inside, a focusable control. */
  focusable?: boolean;
  /** Only when `children` is cut off (an ellipsis or a clamp): the full text of a long name. */
  truncated?: boolean;
};

/**
 * `children` with `hint` in an edge-aware, wrapping tooltip, on hover and on keyboard focus.
 * Replaces native `title` tooltips, which WebKitGTK clips at the window edge.
 */
export function Hint({ hint, children, focusable = true, truncated = false }: Props) {
  const ref = useRef<HTMLButtonElement>(null);
  const [open, setOpen] = useState(false);
  const clipped = useClipped(ref, truncated && Boolean(hint));
  if (!hint) return children;
  const shows = !truncated || clipped();
  return (
    <Tooltip open={open} onOpenChange={(o) => setOpen(o && (!truncated || clipped()))}>
      <TooltipTrigger asChild ref={ref} tabIndex={focusable && shows ? 0 : undefined}>
        {children}
      </TooltipTrigger>
      <TooltipContent side="top" align="end">
        {hint}
      </TooltipContent>
    </Tooltip>
  );
}
