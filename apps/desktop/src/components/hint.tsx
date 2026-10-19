import { type ReactElement, useEffect, useRef, useState } from "react";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { useClipped } from "@/lib/use-clipped";

const FOCUSABLE = "button, a[href], [tabindex]";

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
  // Truncated text inside a control (a card, a row) opens when that control gets keyboard focus.
  useEffect(() => {
    const host =
      truncated && !focusable && hint ? ref.current?.parentElement?.closest(FOCUSABLE) : null;
    if (!host) return;
    const show = () => setOpen(host.matches(":focus-visible") && clipped());
    const hide = () => setOpen(false);
    host.addEventListener("focus", show);
    host.addEventListener("blur", hide);
    return () => {
      host.removeEventListener("focus", show);
      host.removeEventListener("blur", hide);
    };
  }, [truncated, focusable, hint, clipped]);
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
