import { ArrowLeft } from "lucide-react";
import type { ReactNode } from "react";
import { Button } from "@/components/ui/button";
import { WindowControls } from "@/components/window-controls";
import { useNavigation } from "@/lib/navigation";

type Props = { title: ReactNode; subtitle?: ReactNode; back?: boolean; children?: ReactNode };

/**
 * 48px bar shared by every screen that doubles as the window's title bar: title left, actions
 * right, optional back arrow, and window controls where the OS does not draw them. Empty space
 * drags the window (`data-tauri-drag-region` must sit on the element that receives the press).
 */
export function ScreenHeader({ title, subtitle, back, children }: Props) {
  const nav = useNavigation();
  return (
    <header
      data-tauri-drag-region
      className="flex h-12 shrink-0 select-none items-center gap-3 border-b border-stroke px-4"
    >
      {back && (
        <Button variant="ghost" size="icon" aria-label="Back" onClick={nav.back} className="-ml-2">
          <ArrowLeft />
        </Button>
      )}
      <div data-tauri-drag-region className="flex min-w-0 items-baseline gap-2">
        <h1 data-tauri-drag-region className="truncate text-body-lg font-semibold">
          {title}
        </h1>
        {subtitle && (
          <span data-tauri-drag-region className="truncate text-caption text-ink-3">
            {subtitle}
          </span>
        )}
      </div>
      <div data-tauri-drag-region className="ml-auto flex items-center gap-2">
        {children}
      </div>
      <WindowControls />
    </header>
  );
}
