import * as TabsPrimitive from "@radix-ui/react-tabs";
import type { ComponentProps } from "react";
import { cn } from "@/lib/cn";

export const Tabs = TabsPrimitive.Root;

export function TabsList({ className, ...props }: ComponentProps<typeof TabsPrimitive.List>) {
  return (
    <TabsPrimitive.List
      className={cn("inline-flex h-8 items-center gap-1 rounded-md bg-surface-2 p-0.5", className)}
      {...props}
    />
  );
}

/**
 * Radix points `aria-controls` at a `TabsContent` panel; the app's tabs pick a filter or a mode
 * with no such panel, so it is only set when given (the id of what the tab changes).
 */
export function TabsTrigger({
  className,
  "aria-controls": controls,
  ...props
}: ComponentProps<typeof TabsPrimitive.Trigger>) {
  return (
    <TabsPrimitive.Trigger
      aria-controls={controls}
      className={cn(
        "inline-flex h-7 items-center rounded-sm px-2.5 text-body text-ink-2 transition-colors",
        "hover:text-ink data-[state=active]:bg-surface data-[state=active]:text-ink data-[state=active]:shadow-sm",
        "disabled:pointer-events-none disabled:bg-transparent disabled:text-ink-3 disabled:opacity-60",
        className,
      )}
      {...props}
    />
  );
}

export function TabsContent({ className, ...props }: ComponentProps<typeof TabsPrimitive.Content>) {
  return (
    <TabsPrimitive.Content
      className={cn("mt-3 focus-visible:outline-none", className)}
      {...props}
    />
  );
}
