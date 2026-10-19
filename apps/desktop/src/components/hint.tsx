import type { ReactElement } from "react";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";

/**
 * `children` with `hint` (an error or retry reason) in an edge-aware, wrapping tooltip, on hover
 * and on keyboard focus (the trigger is made focusable).
 */
export function Hint({ hint, children }: { hint?: string | null; children: ReactElement }) {
  if (!hint) return children;
  return (
    <Tooltip>
      <TooltipTrigger asChild tabIndex={0}>
        {children}
      </TooltipTrigger>
      <TooltipContent side="top" align="end">
        {hint}
      </TooltipContent>
    </Tooltip>
  );
}
