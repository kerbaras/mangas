import { cva, type VariantProps } from "class-variance-authority";
import type { ComponentProps } from "react";
import { cn } from "@/lib/cn";

const alert = cva(
  "relative grid w-full grid-cols-[0_minmax(0,1fr)] items-start gap-y-0.5 rounded-lg border px-4 py-3 text-body [overflow-wrap:anywhere] has-[>svg]:grid-cols-[16px_minmax(0,1fr)] has-[>svg]:gap-x-3 [&>svg]:size-4 [&>svg]:translate-y-0.5 [&>svg]:text-current",
  {
    variants: {
      variant: {
        default: "border-stroke bg-surface text-ink",
        destructive: "border-accent/40 bg-accent-soft/40 text-accent-text",
      },
    },
    defaultVariants: { variant: "default" },
  },
);

export function Alert({
  className,
  variant,
  ...props
}: ComponentProps<"div"> & VariantProps<typeof alert>) {
  return (
    <div data-slot="alert" role="alert" className={cn(alert({ variant }), className)} {...props} />
  );
}

export function AlertTitle({ className, ...props }: ComponentProps<"div">) {
  return (
    <div
      data-slot="alert-title"
      className={cn("col-start-2 font-medium tracking-tight", className)}
      {...props}
    />
  );
}

export function AlertDescription({ className, ...props }: ComponentProps<"div">) {
  return (
    <div
      data-slot="alert-description"
      className={cn("col-start-2 grid justify-items-start gap-1 text-ink-2", className)}
      {...props}
    />
  );
}
