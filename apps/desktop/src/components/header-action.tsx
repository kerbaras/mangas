import { type ComponentProps, createContext, type ReactNode, type Ref, useContext } from "react";
import { Hint } from "@/components/hint";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/cn";

/** True while the screen header is too narrow for labelled actions (see `ScreenHeader`). */
export const HeaderCompact = createContext(false);

type Props = Omit<ComponentProps<typeof Button>, "children" | "size"> & {
  icon: ReactNode;
  label: string;
  /** Keep the label in a compact header (the screen's main action). */
  keepLabel?: boolean;
  ref?: Ref<HTMLButtonElement>;
};

/**
 * A screen header button: icon and label, or icon only with the label in the shared `Hint`
 * (and as the accessible name) when the header is compact.
 */
export function HeaderAction({ icon, label, keepLabel = false, className, ...props }: Props) {
  const compact = useContext(HeaderCompact) && !keepLabel;
  return (
    <Hint hint={compact ? label : null} focusable={false}>
      <Button
        size="sm"
        aria-label={compact ? label : undefined}
        {...props}
        className={cn("shrink-0 whitespace-nowrap", className)}
      >
        {icon}
        {compact ? null : label}
      </Button>
    </Hint>
  );
}
