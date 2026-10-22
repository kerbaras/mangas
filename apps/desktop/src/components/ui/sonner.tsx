import { Hourglass } from "lucide-react";
import { Toaster as Sonner, type ToasterProps } from "sonner";

/**
 * shadcn's sonner Toaster on the app tokens; follows the OS colour scheme like the rest. Loading
 * toasts (an import, a Connect window) get a still hourglass, not sonner's spinner: an endless
 * spin keeps WebKitGTK repainting without a GPU for as long as the wait lasts. A modal dialog sets
 * `pointer-events: none` on the body; the toaster takes them back so a toast's Retry still works.
 */
export function Toaster(props: ToasterProps) {
  return (
    <Sonner
      theme="system"
      position="bottom-right"
      closeButton
      icons={{ loading: <Hourglass className="size-4 text-ink-3" /> }}
      className="toaster group"
      style={
        {
          "--normal-bg": "var(--color-surface)",
          "--normal-text": "var(--color-ink)",
          "--normal-border": "var(--color-stroke)",
          "--border-radius": "var(--radius-lg)",
          pointerEvents: "auto",
        } as React.CSSProperties
      }
      toastOptions={{
        classNames: {
          toast: "font-sans text-body shadow-lg",
          content: "min-w-0 flex-1",
          title: "font-semibold",
          description: "!text-ink-2 text-caption",
          actionButton: "shrink-0 !bg-accent-fill !text-white !font-medium",
          cancelButton: "!bg-surface-2 !text-ink",
          error: "[&_[data-icon]]:!text-accent",
        },
      }}
      {...props}
    />
  );
}
