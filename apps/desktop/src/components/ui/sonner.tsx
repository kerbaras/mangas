import { Toaster as Sonner, type ToasterProps } from "sonner";

/** shadcn's sonner Toaster on the app tokens; follows the OS colour scheme like the rest. */
export function Toaster(props: ToasterProps) {
  return (
    <Sonner
      theme="system"
      position="bottom-right"
      closeButton
      className="toaster group"
      style={
        {
          "--normal-bg": "var(--color-surface)",
          "--normal-text": "var(--color-ink)",
          "--normal-border": "var(--color-stroke)",
          "--border-radius": "var(--radius-lg)",
        } as React.CSSProperties
      }
      toastOptions={{
        classNames: {
          toast: "font-sans text-body shadow-lg",
          content: "min-w-0 flex-1",
          title: "font-semibold",
          description: "!text-ink-2 text-caption",
          actionButton: "shrink-0 !bg-accent !text-white !font-medium",
          cancelButton: "!bg-surface-2 !text-ink",
          error: "[&_[data-icon]]:!text-accent",
        },
      }}
      {...props}
    />
  );
}
