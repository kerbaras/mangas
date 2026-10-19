import { cva, type VariantProps } from "class-variance-authority";
import type { ComponentProps } from "react";
import { cn } from "@/lib/cn";

const badge = cva(
  "inline-flex h-5 items-center gap-1 rounded-sm px-1.5 text-caption font-medium whitespace-nowrap",
  {
    variants: {
      variant: {
        neutral: "bg-surface-2 text-ink-2",
        accent: "bg-accent-soft text-accent-text",
        outline: "border border-stroke text-ink-2",
      },
    },
    defaultVariants: { variant: "neutral" },
  },
);

type Props = ComponentProps<"span"> & VariantProps<typeof badge>;

export function Badge({ className, variant, ...props }: Props) {
  return <span className={cn(badge({ variant }), className)} {...props} />;
}
