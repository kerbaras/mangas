import type { ComponentProps } from "react";
import { cn } from "@/lib/cn";

/** Native checkbox: the OS draws it, which is what a desktop table expects. */
export function Checkbox({ className, ...props }: ComponentProps<"input">) {
  return (
    <input
      type="checkbox"
      className={cn("size-4 cursor-default rounded-sm accent-accent", className)}
      {...props}
    />
  );
}
